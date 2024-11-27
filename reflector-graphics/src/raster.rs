use crate::simd::*;
use crate::{geom::Point, Color};

#[derive(Copy, Clone)]
pub struct Segment {
    pub p1: Point,
    pub p2: Point,
}

const BITS_PER_BITMASK: usize = u64::BITS as usize;

const PIXELS_PER_BIT: usize = 4;

const PIXELS_PER_BITMASK: usize = PIXELS_PER_BIT * BITS_PER_BITMASK;
const PIXELS_PER_BITMASK_SHIFT: usize = PIXELS_PER_BITMASK.trailing_zeros() as usize;

trait FlipCoords {
    fn winding(value: f32) -> f32;
    fn row(y: usize, height: usize) -> usize;
    fn y_coord(p: Point, height: f32) -> Point;
}

struct PosXPosY;

impl FlipCoords for PosXPosY {
    #[inline(always)]
    fn winding(value: f32) -> f32 {
        value
    }

    #[inline(always)]
    fn row(y: usize, _height: usize) -> usize {
        y
    }

    #[inline(always)]
    fn y_coord(p: Point, _height: f32) -> Point {
        Point::new(p.x, p.y)
    }
}

struct PosXNegY;

impl FlipCoords for PosXNegY {
    #[inline(always)]
    fn winding(value: f32) -> f32 {
        -value
    }

    #[inline(always)]
    fn row(y: usize, height: usize) -> usize {
        height - 1 - y
    }

    #[inline(always)]
    fn y_coord(p: Point, height: f32) -> Point {
        Point::new(p.x, height - p.y)
    }
}

struct NegXPosY;

impl FlipCoords for NegXPosY {
    #[inline(always)]
    fn winding(value: f32) -> f32 {
        value
    }

    #[inline(always)]
    fn row(y: usize, height: usize) -> usize {
        height - 1 - y
    }

    #[inline(always)]
    fn y_coord(p: Point, height: f32) -> Point {
        Point::new(p.x, height - p.y)
    }
}

struct NegXNegY;

impl FlipCoords for NegXNegY {
    #[inline(always)]
    fn winding(value: f32) -> f32 {
        -value
    }

    #[inline(always)]
    fn row(y: usize, _height: usize) -> usize {
        y
    }

    #[inline(always)]
    fn y_coord(p: Point, _height: f32) -> Point {
        Point::new(p.x, p.y)
    }
}

#[derive(Copy, Clone)]
struct Line {
    y1: u16,
    y2: u16,
}

#[derive(Copy, Clone)]
struct Span {
    x: u16,
    width: u16,
}

#[derive(Copy, Clone)]
struct SortedSpan {
    x: u16,
    width: u16,
    data_offset: u32,
}

pub struct Rasterizer {
    width: usize,
    height: usize,
    coverage: Vec<f32>,
    bitmasks_width: usize,
    bitmasks: Vec<u64>,

    lines: Vec<Line>,
    spans: Vec<Span>,
    data: Vec<f32>,

    span_counts: Vec<u32>,
    sorted_spans: Vec<SortedSpan>,

    merged_span_counts: Vec<u32>,
    merged_spans: Vec<Span>,
    merged_data: Vec<f32>,
}

/// Round up to integer number of bitmasks.
fn bitmask_count_for_width(width: usize) -> usize {
    (width + PIXELS_PER_BITMASK - 1) >> PIXELS_PER_BITMASK_SHIFT
}

// On baseline x86_64, f32::floor gets lowered to a function call, so this is significantly faster.
#[inline]
fn floor(x: f32) -> i32 {
    let mut result = x as i32;
    if x < 0.0 {
        result -= 1;
    }
    result
}

impl Rasterizer {
    pub fn new() -> Rasterizer {
        Rasterizer {
            width: 0,
            height: 0,
            coverage: Vec::new(),
            bitmasks_width: 0,
            bitmasks: Vec::new(),

            lines: Vec::new(),
            spans: Vec::new(),
            data: Vec::new(),

            span_counts: Vec::new(),
            sorted_spans: Vec::new(),

            merged_span_counts: Vec::new(),
            merged_spans: Vec::new(),
            merged_data: Vec::new(),
        }
    }

    pub fn set_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;

        let coverage_size = self.width * self.height;
        if self.coverage.len() < coverage_size {
            self.coverage.resize(coverage_size, 0.0);
        }

        self.bitmasks_width = bitmask_count_for_width(self.width);

        let bitmasks_size = self.bitmasks_width * self.height;
        if self.bitmasks.len() < bitmasks_size {
            self.bitmasks.resize(bitmasks_size, 0);
        }

        self.span_counts.resize(height, 0);
    }

    pub fn add_segments(&mut self, segments: &[Segment]) {
        for segment in segments {
            #[allow(clippy::collapsible_else_if)]
            if segment.p1.x < segment.p2.x {
                if segment.p1.y < segment.p2.y {
                    self.add_segment::<PosXPosY>(segment.p1, segment.p2);
                } else {
                    self.add_segment::<PosXNegY>(segment.p1, segment.p2);
                }
            } else {
                if segment.p1.y < segment.p2.y {
                    self.add_segment::<NegXPosY>(segment.p2, segment.p1);
                } else {
                    self.add_segment::<NegXNegY>(segment.p2, segment.p1);
                }
            }
        }
    }

    #[inline(always)]
    fn add_segment<Flip: FlipCoords>(&mut self, p1: Point, p2: Point) {
        let p1 = Flip::y_coord(p1, self.height as f32);
        let p2 = Flip::y_coord(p2, self.height as f32);

        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let dxdy = dx / dy;
        let dydx = dy / dx;

        let mut y = floor(p1.y);
        let mut y_offset = p1.y - y as f32;

        let mut y_end = floor(p2.y);
        let mut y_offset_end = p2.y - y_end as f32;

        let mut x = floor(p1.x);
        let mut x_offset = p1.x - x as f32;

        let mut x_end = floor(p2.x);
        let mut x_offset_end = p2.x - x_end as f32;

        if y >= self.height as i32 {
            return;
        }

        if y_end < 0 {
            return;
        }

        if y < 0 {
            let clip_x = p1.x - dxdy * p1.y;
            x = floor(clip_x);
            x_offset = clip_x - x as f32;

            y = 0;
            y_offset = 0.0;
        }

        if y_end >= self.height as i32 {
            let clip_x = p1.x + dxdy * (self.height as f32 - p1.y);
            x_end = floor(clip_x);
            x_offset_end = clip_x - x as f32;

            y_end = self.height as i32 - 1;
            y_offset_end = 1.0;
        }

        if x >= self.width as i32 {
            return;
        }

        if x < 0 {
            let mut y_split = y_end;
            let mut y_offset_split = y_offset_end;
            if x_end >= 0 {
                let y_clip = p1.y - dydx * p1.x;
                y_split = floor(y_clip).min(self.height as i32 - 1);
                y_offset_split = y_clip - y_split as f32;
            }

            let y1 = Flip::row(y as usize, self.height) as u16;
            let y2 = Flip::row(y_split as usize, self.height) as u16;
            self.lines.push(Line { y1, y2 });

            while y < y_split {
                self.data.push(Flip::winding(1.0 - y_offset));
                self.spans.push(Span { x: 0, width: 1 });

                y += 1;
                y_offset = 0.0;
            }

            self.data.push(Flip::winding(y_offset_split - y_offset));
            self.spans.push(Span { x: 0, width: 1 });

            x = 0;
            x_offset = 0.0;
            y_offset = y_offset_split;
        }

        if x_end < 0 {
            return;
        }

        if x_end >= self.width as i32 {
            x_end = self.width as i32 - 1;
            x_offset_end = 1.0;

            let clip_y = p2.y - dydx * (p2.x - self.width as f32);
            y_end = floor(clip_y);
            y_offset_end = clip_y - y_end as f32;
        }

        let y1 = Flip::row(y as usize, self.height) as u16;
        let y2 = Flip::row(y_end as usize, self.height) as u16;
        self.lines.push(Line { y1, y2 });

        let mut x_offset_next = x_offset + dxdy * (1.0 - y_offset);
        let mut y_offset_next = y_offset + dydx * (1.0 - x_offset);

        while y < y_end {
            let row_start = x as usize;
            let mut carry = 0.0;
            let mut width = 0;
            while y_offset_next < 1.0 {
                let height = Flip::winding(y_offset_next - y_offset);
                let area = 0.5 * height * (1.0 - x_offset);

                self.data.push(carry + area);
                width += 1;
                carry = height - area;

                x += 1;
                x_offset = 0.0;
                x_offset_next -= 1.0;

                y_offset = y_offset_next;
                y_offset_next += dydx;
            }

            let height = Flip::winding(1.0 - y_offset);
            let area = 0.5 * height * (2.0 - x_offset - x_offset_next);

            self.data.push(carry + area);
            width += 1;
            if x as usize + 1 < self.width {
                self.data.push(height - area);
                width += 1;
            }

            self.spans.push(Span {
                x: row_start as u16,
                width,
            });

            x_offset = x_offset_next;
            x_offset_next += dxdy;

            y += 1;
            y_offset = 0.0;
            y_offset_next -= 1.0;
        }

        let row_start = x as usize;
        let mut carry = 0.0;
        let mut width = 0;
        while x < x_end {
            let height = Flip::winding(y_offset_next - y_offset);
            let area = 0.5 * height * (1.0 - x_offset);

            self.data.push(carry + area);
            width += 1;
            carry = height - area;

            x += 1;
            x_offset = 0.0;
            x_offset_next -= 1.0;

            y_offset = y_offset_next;
            y_offset_next += dydx;
        }

        let height = Flip::winding(y_offset_end - y_offset);
        let area = 0.5 * height * (2.0 - x_offset - x_offset_end);

        self.data.push(carry + area);
        width += 1;
        if x as usize + 1 < self.width {
            self.data.push(height - area);
            width += 1;
        }

        self.spans.push(Span {
            x: row_start as u16,
            width,
        });
    }

    pub fn finish(&mut self, color: Color, data: &mut [u32], stride: usize) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            #[cfg(target_feature = "avx2")]
            return self.finish_inner::<Avx2>(color, data, stride);

            #[cfg(all(not(target_feature = "avx2"), target_feature = "sse2"))]
            return self.finish_inner::<Sse2>(color, data, stride);

            #[cfg(not(any(target_feature = "avx2", target_feature = "sse2")))]
            return self.finish_inner::<Scalar>(color, data, stride);
        }

        #[cfg(target_arch = "aarch64")]
        {
            #[cfg(target_feature = "neon")]
            return self.finish_inner::<Neon>(color, data, stride);
        }

        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
        self.finish_inner::<Scalar>(color, data, stride)
    }

    fn finish_inner<A: Arch>(&mut self, color: Color, data: &mut [u32], stride: usize) {
        let mut min_y = self.height - 1;
        let mut max_y = 0;
        for line in &self.lines {
            let (y1, y2) = if line.y1 <= line.y2 {
                (line.y1, line.y2)
            } else {
                (line.y2, line.y1)
            };

            for y in y1 as usize..=y2 as usize {
                self.span_counts[y] += 1;
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
        min_y = min_y.min(max_y);

        let mut accum = 0;
        for span_count in &mut self.span_counts[min_y..max_y + 1] {
            let next = accum + *span_count;
            *span_count = accum;
            accum = next;
        }

        self.sorted_spans.resize(
            self.spans.len(),
            SortedSpan {
                x: 0,
                width: 0,
                data_offset: 0,
            },
        );

        let mut span_index = 0;
        let mut data_offset = 0;
        for line in &self.lines {
            let y1 = line.y1 as usize;
            let y2 = line.y2 as usize;
            if y1 <= y2 {
                for y in y1..=y2 {
                    let span = &self.spans[span_index];
                    let index = &mut self.span_counts[y];
                    self.sorted_spans[*index as usize] = SortedSpan {
                        x: span.x,
                        width: span.width,
                        data_offset,
                    };

                    span_index += 1;
                    *index += 1;
                    data_offset += span.width as u32;
                }
            } else {
                for y in (y2..=y1).rev() {
                    let span = &self.spans[span_index];
                    let index = &mut self.span_counts[y];
                    self.sorted_spans[*index as usize] = SortedSpan {
                        x: span.x,
                        width: span.width,
                        data_offset,
                    };

                    span_index += 1;
                    *index += 1;
                    data_offset += span.width as u32;
                }
            }
        }

        let mut start = 0;
        let mut merged_data_offset = 0;
        for y in min_y..max_y + 1 {
            let end = self.span_counts[y] as usize;

            let mut merged_span_count = 0;
            if start < end {
                self.sorted_spans[start..end].sort_unstable_by_key(|span| span.x);

                merged_span_count = 0;
                let first_span = &self.sorted_spans[start];
                let mut merged_span = Span {
                    x: first_span.x,
                    width: first_span.width,
                };

                let data_start = first_span.data_offset as usize;
                let data_end = data_start + first_span.width as usize;
                self.merged_data.extend(&self.data[data_start..data_end]);

                for span in &self.sorted_spans[start + 1..end] {
                    if span.x <= merged_span.x + merged_span.width {
                        merged_span.width = (merged_span.x + merged_span.width)
                            .max(span.x + span.width)
                            - merged_span.x;

                        self.merged_data
                            .resize(merged_data_offset + merged_span.width as usize, 0.0);

                        let dst = merged_data_offset + (span.x - merged_span.x) as usize;
                        let src = span.data_offset as usize;
                        for i in 0..span.width as usize {
                            self.merged_data[dst + i] += self.data[src + i];
                        }
                    } else {
                        merged_data_offset = self.merged_data.len();
                        self.merged_spans.push(merged_span);
                        merged_span_count += 1;
                        merged_span = Span {
                            x: span.x,
                            width: span.width,
                        };

                        let data_start = span.data_offset as usize;
                        let data_end = data_start + span.width as usize;
                        self.merged_data.extend(&self.data[data_start..data_end]);
                    }
                }
                merged_data_offset = self.merged_data.len();
                self.merged_spans.push(merged_span);
                merged_span_count += 1;
            }

            self.merged_span_counts.push(merged_span_count);
            start = end;
        }

        let a_unit = A::f32::from(color.a() as f32 * (1.0 / 255.0));
        let src = Pixels {
            a: A::f32::from(color.a() as f32),
            r: a_unit * A::f32::from(color.r() as f32),
            g: a_unit * A::f32::from(color.g() as f32),
            b: a_unit * A::f32::from(color.b() as f32),
        };

        let mut start = 0;
        let mut merged_data_offset = 0;
        let mut y = min_y;
        for span_count in &self.merged_span_counts {
            let end = start + *span_count as usize;

            let mut accum = 0.0;
            let mut coverage = 0.0;

            let pixels_start = y * stride;
            let pixels_end = pixels_start + self.width;
            let pixels_row = &mut data[pixels_start..pixels_end];

            let mut x = 0;
            for span in &self.merged_spans[start..end] {
                let next_x = span.x as usize;

                // Composite an interior span (or skip an empty span).
                if next_x > x {
                    if coverage > 254.5 / 255.0 && color.a() == 255 {
                        pixels_row[x..next_x].fill(color.into());
                    } else if coverage > 0.5 / 255.0 {
                        let mut pixels_chunks =
                            pixels_row[x..next_x].chunks_exact_mut(A::u32::LANES);

                        for pixels_slice in &mut pixels_chunks {
                            let mask = A::f32::from(coverage);
                            let dst = Pixels::<A>::unpack(A::u32::load(pixels_slice));
                            dst.blend(src, mask).pack().store(pixels_slice);
                        }

                        let pixels_remainder = pixels_chunks.into_remainder();
                        if !pixels_remainder.is_empty() {
                            let mask = A::f32::from(coverage);
                            let dst = Pixels::unpack(A::u32::load_partial(pixels_remainder));
                            dst.blend(src, mask).pack().store_partial(pixels_remainder);
                        }
                    }
                }

                x = next_x;
                let next_x = ((span.x + span.width) as usize).min(self.width);

                let data_start = merged_data_offset;
                let data_end = merged_data_offset + span.width as usize;

                // Composite an edge span.
                let coverage_slice = &mut self.merged_data[data_start..data_end];
                let mut coverage_chunks = coverage_slice.chunks_exact_mut(A::f32::LANES);

                let pixels_slice = &mut pixels_row[x..next_x];
                let mut pixels_chunks = pixels_slice.chunks_exact_mut(A::u32::LANES);

                for (coverage_chunk, pixels_chunk) in (&mut coverage_chunks).zip(&mut pixels_chunks)
                {
                    let deltas = A::f32::load(coverage_chunk);
                    let accums = A::f32::from(accum) + deltas.prefix_sum();
                    accum = accums.last();
                    let mask = accums.abs().min(A::f32::from(1.0));
                    coverage = mask.last();

                    let dst = Pixels::<A>::unpack(A::u32::load(pixels_chunk));
                    dst.blend(src, mask).pack().store(pixels_chunk);
                }

                let coverage_remainder = coverage_chunks.into_remainder();
                let pixels_remainder = pixels_chunks.into_remainder();
                if !pixels_remainder.is_empty() && !coverage_remainder.is_empty() {
                    let deltas = A::f32::load_partial(coverage_remainder);
                    let accums = A::f32::from(accum) + deltas.prefix_sum();
                    accum = accums.last();
                    let mask = accums.abs().min(A::f32::from(1.0));
                    coverage = mask.last();

                    let dst = Pixels::<A>::unpack(A::u32::load_partial(pixels_remainder));
                    dst.blend(src, mask).pack().store_partial(pixels_remainder);
                }

                merged_data_offset += span.width as usize;
                x = next_x;
            }

            let next_x = self.width;

            // Composite an interior span (or skip an empty span).
            if next_x > x {
                if coverage > 254.5 / 255.0 && color.a() == 255 {
                    pixels_row[x..next_x].fill(color.into());
                } else if coverage > 0.5 / 255.0 {
                    let mut pixels_chunks = pixels_row[x..next_x].chunks_exact_mut(A::u32::LANES);

                    for pixels_slice in &mut pixels_chunks {
                        let mask = A::f32::from(coverage);
                        let dst = Pixels::<A>::unpack(A::u32::load(pixels_slice));
                        dst.blend(src, mask).pack().store(pixels_slice);
                    }

                    let pixels_remainder = pixels_chunks.into_remainder();
                    if !pixels_remainder.is_empty() {
                        let mask = A::f32::from(coverage);
                        let dst = Pixels::unpack(A::u32::load_partial(pixels_remainder));
                        dst.blend(src, mask).pack().store_partial(pixels_remainder);
                    }
                }
            }

            start = end;
            y += 1;
        }

        self.lines.clear();
        self.spans.clear();
        self.data.clear();

        self.merged_span_counts.clear();
        self.merged_spans.clear();
        self.merged_data.clear();

        self.span_counts[min_y..max_y + 1].fill(0);
    }
}

struct Pixels<A: Arch> {
    a: A::f32,
    r: A::f32,
    g: A::f32,
    b: A::f32,
}

impl<A: Arch> Clone for Pixels<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: Arch> Copy for Pixels<A> {}

impl<A: Arch> Pixels<A> {
    #[inline]
    fn unpack(data: A::u32) -> Self {
        Pixels {
            a: A::f32::from((data >> 24) & A::u32::from(0xFF)),
            r: A::f32::from((data >> 16) & A::u32::from(0xFF)),
            g: A::f32::from((data >> 8) & A::u32::from(0xFF)),
            b: A::f32::from((data >> 0) & A::u32::from(0xFF)),
        }
    }

    #[inline]
    fn pack(self) -> A::u32 {
        let a = A::u32::from(self.a);
        let r = A::u32::from(self.r);
        let g = A::u32::from(self.g);
        let b = A::u32::from(self.b);

        (a << 24) | (r << 16) | (g << 8) | (b << 0)
    }

    #[inline]
    fn blend(self, src: Self, mask: A::f32) -> Self {
        let inv_a = A::f32::from(1.0) - mask * A::f32::from(1.0 / 255.0) * src.a;
        Pixels {
            a: mask * src.a + inv_a * self.a,
            r: mask * src.r + inv_a * self.r,
            g: mask * src.g + inv_a * self.g,
            b: mask * src.b + inv_a * self.b,
        }
    }
}

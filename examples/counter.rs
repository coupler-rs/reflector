use reflector::elems::{Padding, Row, Text};
use reflector::graphics::Font;
use reflector::{App, Build, Size, WindowOptions};

fn build() -> impl Build {
    let font = Font::from_bytes(
        include_bytes!("../reflector-graphics/examples/res/SourceSansPro-Regular.otf"),
        0,
    )
    .unwrap();

    Row::new()
        .child(Padding::new(5.0, Text::new("text", font.clone(), 16.0)))
        .child(Padding::new(5.0, Text::new("text", font.clone(), 16.0)))
        .child(Padding::new(5.0, Text::new("text", font.clone(), 16.0)))
        .child(Padding::new(5.0, Text::new("text", font.clone(), 16.0)))
}

fn main() {
    let app = App::new().unwrap();

    WindowOptions::new()
        .title("window")
        .size(Size::new(512.0, 512.0))
        .open(&app, build())
        .unwrap();

    app.run().unwrap();
}

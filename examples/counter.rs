use reflector::elems::Text;
use reflector::graphics::Font;
use reflector::{App, Size, WindowOptions};

fn main() {
    let app = App::new().unwrap();

    let font = Font::from_bytes(
        include_bytes!("../reflector-graphics/examples/res/SourceSansPro-Regular.otf"),
        0,
    )
    .unwrap();

    WindowOptions::new()
        .title("window")
        .size(Size::new(512.0, 512.0))
        .open(&app, Text::new("text", font, 16.0))
        .unwrap();

    app.run().unwrap();
}

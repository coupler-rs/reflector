use reflector::elems::{Button, Row, Text};
use reflector::graphics::Font;
use reflector::{App, BuildElem, Size, WindowOptions};

fn build() -> impl BuildElem {
    let font = Font::from_bytes(
        include_bytes!("../reflector-graphics/examples/res/SourceSansPro-Regular.otf"),
        0,
    )
    .unwrap();

    Row::new(5.0)
        .child(Text::new("text", font.clone(), 16.0))
        .child(Button::new(Text::new("button", font.clone(), 16.0)).action(|| println!("click")))
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

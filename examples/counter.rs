use reflector::{App, Size, WindowOptions};

fn main() {
    let app = App::new().unwrap();

    WindowOptions::new()
        .title("window")
        .size(Size::new(512.0, 512.0))
        .open(&app)
        .unwrap();

    app.run().unwrap();
}

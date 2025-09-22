use term_render::{self, event_handler::KeyCode};
use term_render::widget_impls::WidgetBuilder;

fn app_callback(app: &mut term_render::App, data: &mut AppData) -> Result<bool, ()> {
    if app.events.read().contains_key_code(KeyCode::Return) {
        return Ok(true);
    }
    if data.time.elapsed().as_secs_f64() > 15.0 { return Ok(true); }
    Ok(false)
}

struct AppData { pub time: std::time::Instant }

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> tokio::io::Result<()> {
    let mut app = term_render::App::new()?;
    let data = AppData { time: std::time::Instant::now() };
    let mut scene = term_render::widget::Scene::new();
    let (widget, window) = term_render::widget_impls::StaticWidgetBuilder::builder(String::from("name"))
        .with_border(true)
        .with_renderer(Box::new(|_size, _position| { Some(vec![]) }))
        .with_position((10, 10))
        .with_size((50, 10))
        .build(&app.area.read()).unwrap();
    scene.add_widget(widget, window, &mut *app.renderer.write()).unwrap();
    app.scene = Some(scene);
    app.run(data, |data, app_instance| { app_callback(app_instance, data) }).await.unwrap();
    Ok(())
}

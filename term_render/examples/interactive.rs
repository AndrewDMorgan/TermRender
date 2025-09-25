use term_render::{self, event_handler::KeyCode};
use term_render::render::{Colorize, ColorType};
use term_render::widget_impls::WidgetBuilder;
use term_render::render::Span;
use term_render::color;
use term_render::widget::Widget;

// this acts as the callback that is called every frame
// this is the entry point and any logic needs to branch out from here
fn app_callback(app: &mut term_render::App<AppData>, data: &mut AppData) -> Result<bool, ()> {
    // place app logic here
    if app.events.read().contains_key_code(KeyCode::Return) {
        return Ok(true);
    }
    if data.time.elapsed().as_secs_f64() > 15.0 {
        return Ok(true);
    }
    
    Ok(false)  // return true to exit the app
}

// defining the application data structure
// this can contain any data you want to use in the app callback
struct AppData {
    pub time: std::time::Instant,
}

// the main function has to be async to enable the use of async tasks further down the road (mainly under the hood)
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> tokio::io::Result<()> {
    // creating the application instance (this will initialize the renderer and event handler)
    let mut app = term_render::App::new()?;
    
    // defining the application data (stored separately from the app instance, but linked from the instance)
    let data = AppData {
        time: std::time::Instant::now(),
    };
    
    let mut scene = term_render::widget::Scene::new();
    
    let (widget, window) =
        term_render::widget_impls::DynamicWidgetBuilder::<AppData>::builder(String::from("button"))
            .with_border(true)
            .with_renderer(Box::new(|_size, _position| {
                Some(vec![Span::from_tokens(vec![color!("I am thing!")])])
            }))
            .with_position((10, 10))
            .with_size((50, 10))
            // realistically, instead of writing one long closure, the logic could be placed in a function which is called
            .with_update_handler(Box::new(|widget, _data, app: &mut term_render::App<AppData>, scene| {
                let widget_index = scene.get_widget_index(widget.get_window_ref()).unwrap_or(0);
                // checking if the widget was clicked
                let pressed = if let Some(event) = &app.events.read().mouse_event {
                    if event.event_type == term_render::event_handler::MouseEventType::Left &&
                       event.state == term_render::event_handler::MouseState::Press &&
                       widget.is_collided(event.position) {
                        // the button/widget was clicked
                        !scene.is_click_blocked(widget_index, event.position).unwrap_or(false)
                    } else {  false  }
                } else {  false  };
                // checking if the widegt contains a child, otherwise creating one (for a pop up)
                if !pressed {  return  }
                if widget.get_children_indexes().is_empty() {
                    // creating a new widget
                    let (mut widget_child, window) = term_render::widget_impls::ButtonWidgetBuilder::<AppData>::builder(String::from("popup"))
                        .with_border(true)
                        .with_renderer(Box::new(|_size, _position| {
                            Some(vec![Span::from_tokens(vec![color!("This is a popup!", Red)])])
                        }))
                        .with_update_handler(Box::new(|widget, data, app: &mut term_render::App<AppData>, scene, state| {
                            let widget_index = scene.get_widget_index(widget.get_window_ref()).unwrap_or(0);
                            // checking if the widget was clicked
                            let mut pressed = state == &term_render::widget_impls::ButtonState::Pressed(term_render::event_handler::MouseEventType::Left);
                            if let Some(event) = &app.events.read().mouse_event {
                                pressed &= !scene.is_click_blocked(widget_index, event.position).unwrap_or(false);
                            }
                            // checking if the widget contains a child, otherwise creating one (for a pop up)
                            if !pressed {  return  }
                            if widget.get_children_indexes().is_empty() {
                                // creating a new widget
                                let (mut widget_child, window) = term_render::widget_impls::StaticWidgetBuilder::<AppData>::builder(String::from("popup_final"))
                                    .with_border(true)
                                    .with_renderer(Box::new(|_size, _position| {
                                        Some(vec![Span::from_tokens(vec![color!("This is another popup!", Red)])])
                                    }))
                                    .with_dynamic_position((18, 7), (0.1, 0.1))
                                    .with_dynamic_size((35, 14), (0.1, 0.1))
                                    .with_depth(1)
                                    .build(&app.area.read()).unwrap();
                                widget_child.set_parent_index(scene.get_widget_index(String::from("popup")));
                                // adding the child to the current widget
                                scene.add_widget(widget_child, window, &mut *app.renderer.write()).unwrap();
                            } else {
                                // removing the child
                                scene.remove_widget(scene.get_widget_index(String::from("popup_final")).unwrap_or(0), &mut *app.renderer.write()).unwrap();
                            }
                        }))
                        .with_position((15, 5))
                        .with_size((30, 12))
                        .with_depth(1)
                        .build(&app.area.read()).unwrap();
                    widget_child.set_parent_index(scene.get_widget_index(String::from("button")));
                    // adding the child to the current widget
                    scene.add_widget(widget_child, window, &mut *app.renderer.write()).unwrap();
                } else {
                    // removing the child
                    scene.remove_widget(scene.get_widget_index(String::from("popup")).unwrap_or(0), &mut *app.renderer.write()).unwrap();
                }
            }))
            .build(&app.area.read()).unwrap();
    scene.add_widget(widget, window, &mut *app.renderer.write()).unwrap();
    app.scene = Some(scene);
    
    // running the application with the provided callback function
    app.run(data, |data, app_instance: &mut term_render::App<AppData>| {
        app_callback(app_instance, data)
    }).await.unwrap();
    
    Ok(())
}

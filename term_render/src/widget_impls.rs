use crate::{event_handler, SendSync};
use crate::widget::*;

pub trait WidgetBuilder: Default {
    fn build(self, display_area: &crate::render::Rect) -> Result<(Box<dyn Widget>, crate::render::Window), WidgetBuilderError>;
    fn with_position(self, position: (u16, u16)) -> Self;
    fn with_size(self, size: (u16, u16)) -> Self;
    fn with_dynamic_position(self, position_offset: (i16, i16), position_area_percent: (f32, f32)) -> Self;
    fn with_dynamic_size(self, size_offset: (i16, i16), size_area_percent: (f32, f32)) -> Self;
    fn with_border(self, border: bool) -> Self;
    fn with_title(self, title: String) -> Self;
    fn with_depth(self, depth: u16) -> Self;
    fn with_renderer(self, renderer: Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>) -> Self;
    fn builder(name: String) -> Self;
    fn with_sap(self, sap: SizeAndPosition) -> Self;
}

#[derive(Default)]
pub struct SizeAndPosition {
    pub size_offset: (i16, i16),
    pub position_offset: (i16, i16),
    
    pub size_area_percent: (f32, f32),  // percentage of the terminal area (0.5 is the center)
    pub position_area_percent: (f32, f32),  // percentage of the terminal area (0.5 is the center)
}

impl SizeAndPosition {
    /// Creates an instance with a constant size and position, ignoring the resized area of the terminal.
    pub fn new_static(size: (u16, u16), position: (u16, u16)) -> SizeAndPosition {
        SizeAndPosition {
            size_offset: (size.0 as i16, size.1 as i16),
            position_offset: (position.0 as i16, position.1 as i16),
            size_area_percent: (0.0, 0.0),
            position_area_percent: (0.0, 0.0),
        }
    }
    
    /// Creates an instance with a dynamic size and position, based on the resized area of the terminal, and a constant offset.
    pub fn new_dynamic(size_offset: (i16, i16), position_offset: (i16, i16), size_area_percent: (f32, f32), position_area_percent: (f32, f32)) -> SizeAndPosition {
        SizeAndPosition {
            size_offset,
            position_offset,
            size_area_percent,
            position_area_percent,
        }
    }
    
    pub fn get_size_and_position(&self, area: &crate::render::Rect) -> ((u16, u16), (u16, u16)) {
        let width = (((area.width as f32) * self.size_area_percent.0) as i16 + self.size_offset.0) as u16;
        let height = (((area.height as f32) * self.size_area_percent.1) as i16 + self.size_offset.1) as u16;
        
        let x = (((area.width as f32) * self.position_area_percent.0) as i16 + self.position_offset.0) as u16;
        let y = (((area.height as f32) * self.position_area_percent.1) as i16 + self.position_offset.1) as u16;
        
        ((width, height), (x, y))
    }
}

#[derive(Debug)]
pub struct WidgetBuilderError {
    details: String,
}

#[derive(Default)]
pub struct StaticWidget {
    children: Vec<usize>,
    name: String,
    parent_index: Option<usize>,
    depth: Option<u16>,
    boarder: bool,
    title: Option<String>,
    
    // this should be an easy, lightweight, and changeable way to get the size and position
    // it needs to be light enough to be changed to something new if the state of the app changes
    // (such as a menu opening and shifting things).
    pub size_and_position: SizeAndPosition,
    
    // takes the size and position in, and returns the vector of spans to render
    // this is a function object, allowing for capturing of state if desired
    pub render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,
}

impl StaticWidget {
    pub fn new(name: String,
               size_and_position: SizeAndPosition,
               render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,
               depth: u16,
               display_area: &crate::render::Rect,
    ) -> Result<(StaticWidget, crate::render::Window), WidgetErr> {
        let (size, position) = size_and_position.get_size_and_position(display_area);
        if size.0 == 0 || size.1 == 0 || position.0 == 0 || position.1 == 0 {
            return Err(WidgetErr::new("Size or position cannot be zero"))
        }
        let window = crate::render::Window::new(position, depth, size);
        
        let widget = StaticWidget {
            children: vec![],
            name,
            parent_index: None,
            depth: Some(depth),
            size_and_position,
            render_function,
            boarder: false,
            title: None,
        };
        
        Ok((widget, window))
    }
}

impl WidgetBuilder for StaticWidget {
    fn build(self, display_area: &crate::render::Rect) -> Result<(Box<dyn Widget>, crate::render::Window), WidgetBuilderError> {
        let (position, size) = self.size_and_position.get_size_and_position(display_area);
        if size.0 == 0 || size.1 == 0 || position.0 == 0 || position.1 == 0 {
            return Err(WidgetBuilderError { details: String::from("Position and/or size cannot be zero when building a new widget or window.") })
        }
        let depth = self.depth.as_ref().unwrap_or(&0u16);
        let mut window = crate::render::Window::new(position, *depth, size);
        if self.boarder {  window.bordered();  }
        if let Some(title) = &self.title {  window.titled(title.clone());  }
        Ok((Box::new(self), window))
    }
    
    fn with_position(mut self, position: (u16, u16)) -> Self {
        self.size_and_position.position_offset = (position.0 as i16, position.1 as i16);
        self
    }
    
    fn with_size(mut self, size: (u16, u16)) -> Self {
        self.size_and_position.size_offset = (size.0 as i16, size.1 as i16);
        self
    }
    
    fn with_dynamic_position(mut self, position_offset: (i16, i16), position_area_percent: (f32, f32)) -> Self {
        self.size_and_position.position_offset = position_offset;
        self.size_and_position.position_area_percent = position_area_percent;
        self
    }
    
    fn with_dynamic_size(mut self, size_offset: (i16, i16), size_area_percent: (f32, f32)) -> Self {
        self.size_and_position.size_offset = size_offset;
        self.size_and_position.size_area_percent = size_area_percent;
        self
    }
    
    fn with_border(mut self, border: bool) -> Self {
        self.boarder = true;
        self
    }
    
    fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
    
    fn with_depth(mut self, depth: u16) -> Self {
        self.depth = Some(depth);
        self
    }
    
    fn with_renderer(mut self, renderer: Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>) -> Self {
        self.render_function = Some(renderer);
        self
    }
    
    fn builder(name: String) -> Self {
        Self {
            children: vec![],
            name,
            parent_index: None,
            depth: None,
            size_and_position: SizeAndPosition::default(),
            render_function: None,
            boarder: false,
            title: None,
        }
    }
    
    fn with_sap(mut self, sap: SizeAndPosition) -> Self {
        self.size_and_position = sap;
        self
    }
}

impl Widget for StaticWidget {
    fn get_window_ref(&self) -> String {
        self.name.clone()
    }
    
    // for handling updates (a static widget would just have this empty)
    fn update_with_events(&mut self, _events: &SendSync<event_handler::KeyParser>) {
        // the static widget doesn't need to change
    }
    
    /// Updates the underlying window.
    fn update_render(&mut self, window: &mut crate::render::Window, area: &crate::render::Rect) {
        // only needs to change with size
        let (size, position) = self.size_and_position.get_size_and_position(area);
        let render = (*self.render_function.as_ref().unwrap())(size, position);
        window.resize(size);
        window.r#move(position);
        if let Some(render) = render {
            window.try_update_lines(render);
        }
    }
    
    fn get_children_indexes(&self) -> Vec<usize> {
        self.children.clone()
    }
    fn add_child_index(&mut self, index: usize) {
        self.children.push(index);
    }
    fn remove_child_index(&mut self, index: usize) {
        self.children.remove(index);
    }
    fn clear_children_indexes(&mut self) {
        self.children.clear();
    }
    
    fn get_parent_index(&self) -> Option<usize> {
        self.parent_index
    }
    fn set_parent_index(&mut self, index: Option<usize>) {
        self.parent_index = index;
    }
}



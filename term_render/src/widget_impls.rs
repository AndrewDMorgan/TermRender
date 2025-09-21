use crate::{event_handler, SendSync};
use crate::widget::*;

/// A builder trait for constructing widgets with a fluent interface.
/// Provides methods to configure widget properties like position, size, borders, and rendering behavior.
/// Implementors should provide a `build` method that creates the widget and its associated window.
/// Implementors often *won't* implement `Widget`, but the returned type from `build` will.
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

/// Represents a widget's size and position configuration, supporting both static and dynamic layouts.
/// Dynamic layouts are calculated relative to the terminal area with optional fixed offsets.
/// Static layouts are solely based on the provided constant, and as such won't change if the terminal
/// is resized.
#[derive(Default)]
pub struct SizeAndPosition {
    pub size_offset: (i16, i16),
    pub position_offset: (i16, i16),
    
    pub size_area_percent: (f32, f32),  // percentage of the terminal area (0.5 is the center)
    pub position_area_percent: (f32, f32),  // percentage of the terminal area (0.5 is the center)
}

impl SizeAndPosition {
    /// Creates a SizeAndPosition with fixed size and position that doesn't respond to terminal resizing.
    /// # Arguments
    /// * `size` - Fixed dimensions (width, height) in characters
    /// * `position` - Fixed position (x, y) in character coordinates
    ///
    /// *- The relative size and position to the terminal's area are set to 0.*
    pub fn new_static(size: (u16, u16), position: (u16, u16)) -> SizeAndPosition {
        SizeAndPosition {
            size_offset: (size.0 as i16, size.1 as i16),
            position_offset: (position.0 as i16, position.1 as i16),
            size_area_percent: (0.0, 0.0),
            position_area_percent: (0.0, 0.0),
        }
    }
    
    /// Creates a SizeAndPosition that dynamically adjusts based on terminal size.
    /// # Arguments
    /// * `size_offset` - Fixed size adjustment (width, height) in characters
    /// * `position_offset` - Fixed position adjustment (x, y) in character coordinates
    /// * `size_area_percent` - Size as percentage of terminal area (width%, height%) in the range [0, 1]
    /// * `position_area_percent` - Position as percentage of terminal area (x%, y%) in the range [0, 1]
    pub fn new_dynamic(size_offset: (i16, i16), position_offset: (i16, i16), size_area_percent: (f32, f32), position_area_percent: (f32, f32)) -> SizeAndPosition {
        SizeAndPosition {
            size_offset,
            position_offset,
            size_area_percent,
            position_area_percent,
        }
    }
    
    /// Calculates the actual size and position based on the current terminal area.
    /// # Arguments
    /// * `area` - The current terminal display area
    /// # Returns
    /// Tuple containing ((width, height), (x, y)) coordinates
    ///  - *Note: static layout configurations will always return the values regardless of the inputted area.*
    pub fn get_size_and_position(&self, area: &crate::render::Rect) -> ((u16, u16), (u16, u16)) {
        let width = (((area.width as f32) * self.size_area_percent.0) as i16 + self.size_offset.0) as u16;
        let height = (((area.height as f32) * self.size_area_percent.1) as i16 + self.size_offset.1) as u16;
        
        let x = (((area.width as f32) * self.position_area_percent.0) as i16 + self.position_offset.0) as u16;
        let y = (((area.height as f32) * self.position_area_percent.1) as i16 + self.position_offset.1) as u16;
        
        ((width, height), (x, y))
    }
}

/// Error type for widget building operations, containing details about what went wrong.
#[derive(Debug)]
pub struct WidgetBuilderError {
    details: String,
}

impl std::fmt::Display for WidgetBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WidgetBuilderError: {}", self.details)
    }
}

/// Builder for creating StaticWidget instances with a fluent interface.
/// Maintains configuration state until build() is called to create the actual widget.
/// `StaticWidgetBuilder` is an example of an implementation of `WidgetBuilder`, where
/// the struct doesn't implement `Widget`.
#[derive(Default)]
pub struct StaticWidgetBuilder {
    children: Vec<usize>,
    name: String,
    parent_index: Option<usize>,
    depth: Option<u16>,
    boarder: bool,
    title: Option<String>,
    pub size_and_position: SizeAndPosition,
    pub render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,
}

/// Implementations for the methods in `WidgetBuilder`.
impl WidgetBuilder for StaticWidgetBuilder {
    /// Constructs a `StaticWidget`, an implementor of `Widget`, given the parameters.
    /// Validates that size and position are non-zero before creating the widget.
    /// The method takes in a reference to the terminal's current area/size.
    /// # Example:
    /// ```
    /// use term_render::widget_impls::{StaticWidgetBuilder, WidgetBuilder};
    /// use term_render::render::Rect;
    /// let (widget, window) = StaticWidgetBuilder::builder(String::new())
    ///     .build(&Rect::default())  // replace &Rect with the actual terminal size (such as `&app.area.read()`)
    ///     .expect("Invalid widget position or size.");
    /// ```
    fn build(self, display_area: &crate::render::Rect) -> Result<(Box<dyn Widget>, crate::render::Window), WidgetBuilderError> {
        let (position, size) = self.size_and_position.get_size_and_position(display_area);
        if size.0 == 0 || size.1 == 0 || position.0 == 0 || position.1 == 0 {
            return Err(WidgetBuilderError { details: String::from("Position and/or size cannot be zero when building a new widget or window.") })
        }
        let depth = self.depth.as_ref().unwrap_or(&0u16);
        let mut window = crate::render::Window::new(position, *depth, size);
        if self.boarder {  window.bordered();  }
        if let Some(title) = &self.title {  window.titled(title.clone());  }
        Ok((Box::new(StaticWidget {
            children: self.children,
            name: self.name,
            parent_index: self.parent_index,
            size_and_position: self.size_and_position,
            render_function: self.render_function,
        }), window))
    }
    
    /// Sets the widget's fixed position (static layout).
    /// Retrains the dynamic proportions of any dynamic positioning configuration already in place.
    fn with_position(mut self, position: (u16, u16)) -> Self {
        self.size_and_position.position_offset = (position.0 as i16, position.1 as i16);
        self
    }
    
    /// Sets the widget's fixed position (static layout).
    /// Retrains the dynamic proportions of any dynamic size configuration already in place.
    fn with_size(mut self, size: (u16, u16)) -> Self {
        self.size_and_position.size_offset = (size.0 as i16, size.1 as i16);
        self
    }
    
    /// Configures dynamic positioning based on terminal size with a fixed offset.
    fn with_dynamic_position(mut self, position_offset: (i16, i16), position_area_percent: (f32, f32)) -> Self {
        self.size_and_position.position_offset = position_offset;
        self.size_and_position.position_area_percent = position_area_percent;
        self
    }
    
    /// Configures dynamic sizing based on terminal size with a fixed offset.
    fn with_dynamic_size(mut self, size_offset: (i16, i16), size_area_percent: (f32, f32)) -> Self {
        self.size_and_position.size_offset = size_offset;
        self.size_and_position.size_area_percent = size_area_percent;
        self
    }
    
    /// Sets whether the widget should have a border. By default, all widgets are borderless.
    fn with_border(mut self, border: bool) -> Self {
        self.boarder = border;
        self
    }
    
    /// Sets the widget's title (displayed in border if enabled; invisible otherwise).
    fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
    
    /// Assigns a depth to the widget. Higher values represent UI elements that are stacked further
    /// ontop of other elements. By default, the builder instance has a depth of None, which corresponds
    /// to 0 (root depth). This is fine as long as it isn't trying to appear ontop of other elements, such
    /// as for a pop-up.
    fn with_depth(mut self, depth: u16) -> Self {
        self.depth = Some(depth);
        self
    }
    
    /// Sets the rendering closure that generates content for the widget.
    /// The closure receives size and position parameters and returns an optional vector of type `Span`.
    /// By default, there is no renderer, leaving the widget empty (apart from stylization like a border or title).
    /// The closure is a boxed closure that takes in `(size: (u16, u16), position: (u16, u16))`. This closure
    /// can capture local context to allow for easier dynamic variations between widgets with minimal boilerplate.
    /// #Example:
    /// ```
    /// use term_render::widget_impls::{StaticWidgetBuilder, WidgetBuilder};
    /// use term_render::render::Rect;
    ///
    /// // the closure can capture local variables to reduce boilerplate
    /// let closure = Box::new(move |size, position| {
    ///     None  // this will leave the widget un-updated
    /// });
    /// let (widget, window) = StaticWidgetBuilder::builder(String::new())
    ///     .with_renderer(closure)
    ///     .build(&Rect::default())
    ///     .unwrap();
    /// ```
    fn with_renderer(mut self, renderer: Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>) -> Self {
        self.render_function = Some(renderer);
        self
    }
    
    /// Generates a new builder instance with a provided unique name identifier.
    /// By default, size_and_position are no compatible and require the user to
    /// provide them using the other `WidgetBuilder` trait functions.
    /// # Example:
    /// ```
    /// use term_render::widget_impls::{StaticWidgetBuilder, WidgetBuilder};
    /// let builder = StaticWidgetBuilder::builder(String::from("Widget Name"));
    /// ```
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
    
    /// Sets the SizeAndPosition configuration directly.
    fn with_sap(mut self, sap: SizeAndPosition) -> Self {
        self.size_and_position = sap;
        self
    }
}

/// A widget that renders static content using a provided closure (i.e.
/// a title box or description).
/// Suitable for content that doesn't change frequently or in response to events.
/// `StaticWidgetBuilder` is the associated builder for creating instances of this widget.
#[derive(Default)]
pub struct StaticWidget {
    children: Vec<usize>,
    name: String,
    parent_index: Option<usize>,
    
    // this should be an easy, lightweight, and changeable way to get the size and position
    // it needs to be light enough to be changed to something new if the state of the app changes
    // (such as a menu opening and shifting things).
    pub size_and_position: SizeAndPosition,
    
    // takes the size and position in, and returns the vector of spans to render
    // this is a function object, allowing for capturing of state if desired
    pub render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,
}

impl StaticWidget {
    /// Creates a new StaticWidget and its associated window.
    /// # Arguments
    /// * `name` - Unique identifier for the widget
    /// * `size_and_position` - Configuration for size and positioning
    /// * `render_function` - Optional closure that renders content given size and position
    /// * `depth` - Z-index for rendering order
    /// * `display_area` - Current terminal area for initial size/position calculation
    /// # Returns
    /// Result containing the widget and window, or error if dimensions are invalid
    ///
    /// *When possible, an implementation of `WidgetBuilder` should be used instead, both for safety,
    /// simplicity, and consistency.*
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
            size_and_position,
            render_function,
        };
        
        Ok((widget, window))
    }
}

/// Implementation of the methods for StaticWidget
impl Widget for StaticWidget {
    /// Returns the widget's name as an identifier. The rendering backend relies
    /// on `String` names instead of widgets.
    fn get_window_ref(&self) -> String {
        self.name.clone()
    }
    
    // for handling updates (a static widget would just have this empty)
    /// Handles event updates (no-op for static widgets as they don't respond to events)
    fn update_with_events(&mut self, _events: &SendSync<event_handler::KeyParser>) {
        // the static widget doesn't need to change
    }
    
    /// Updates the widget's rendering based on current size and position.
    /// Called automatically during render passes.
    /// If `Some(render_closure)` is provided, that closure will be called.
    /// If the closure returns `Some(Vec<Span>)`, then the rendered content will be set as such.
    fn update_render(&mut self, window: &mut crate::render::Window, area: &crate::render::Rect) -> bool {
        // only needs to change with size
        let (size, position) = self.size_and_position.get_size_and_position(area);
        window.resize(size);
        window.r#move(position);
        if let Some(render_function) = &self.render_function {
            if let Some(render) = render_function(size, position) {
                return window.try_update_lines(render);
            }
        } false
    }
    
    /// Returns the indices of child widgets in the scene graph.
    fn get_children_indexes(&self) -> Vec<usize> {
        self.children.clone()
    }
    
    /// Adds a child widget index to this widget.
    fn add_child_index(&mut self, index: usize) {
        self.children.push(index);
    }
    
    /// Removes a child widget index from this widget
    fn remove_child_index(&mut self, index: usize) {
        self.children.remove(index);
    }
    
    /// Clears all child widget indices from this widget
    fn clear_children_indexes(&mut self) {
        self.children.clear();
    }
    
    /// Returns the parent widget index if one exists, otherwise None.
    fn get_parent_index(&self) -> Option<usize> {
        self.parent_index
    }
    
    /// Sets the parent widget index for this widget, or None for a root node.
    fn set_parent_index(&mut self, index: Option<usize>) {
        self.parent_index = index;
    }
}



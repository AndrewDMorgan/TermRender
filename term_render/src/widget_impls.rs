use crate::widget::*;

/// A builder trait for constructing widgets with a fluent interface.
/// Provides methods to configure widget properties like position, size, borders, and rendering behavior.
/// Implementors should provide a `build` method that creates the widget and its associated window.
/// Implementors often *won't* implement `Widget`, but the returned type from `build` will.
pub trait WidgetBuilder<C> {
    /// Constructs the widget and its associated window based on the current configuration.
    /// It's advised, but not required, the implementor doesn't implement `Widget`.
    /// The returned type does have to implement `Widget`.
    fn build(self, display_area: &crate::render::Rect) -> Result<(Box<dyn Widget<C>>, crate::render::Window), WidgetBuilderError>;
    /// Sets the widget's position
    fn with_position(self, position: (u16, u16)) -> Self;
    /// Sets the widget's size
    fn with_size(self, size: (u16, u16)) -> Self;
    /// Sets the widget's dynamic position
    fn with_dynamic_position(self, position_offset: (i16, i16), position_area_percent: (f32, f32)) -> Self;
    /// Sets the widget's dynamic size
    fn with_dynamic_size(self, size_offset: (i16, i16), size_area_percent: (f32, f32)) -> Self;
    /// Sets if the widget has a border or not
    fn with_border(self, border: bool) -> Self;
    /// Sets the widget's title (displayed in border if enabled; invisible otherwise)
    fn with_title(self, title: String) -> Self;
    /// Sets the widget's depth (z-index)
    fn with_depth(self, depth: u16) -> Self;
    /// Sets the widget's custom renderer closure
    fn with_renderer(self, renderer: Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>) -> Self;
    /// Creates a new builder instance with the provided unique name identifier.
    /// It's recommended that the identifying name is as inputed, and not
    /// modified, to avoid conflicts if the user manually attempts to access the widget.
    fn builder(name: String) -> Self;
    /// Sets the widget's SizeAndPosition configuration directly.
    fn with_sap(self, sap: SizeAndPosition) -> Self;
    /// Sets the widget's update handler closure. This closure is called during event updates.
    fn with_update_handler(self, _handler: Box<dyn Fn(&mut dyn Widget<C>, &mut C, &mut crate::App<C>, &mut crate::widget::Scene<C>)>) -> Self;
}

/// Represents a widget's size and position configuration, supporting both static and dynamic layouts.
/// Dynamic layouts are calculated relative to the terminal area with optional fixed offsets.
/// Static layouts are solely based on the provided constant, and as such won't change if the terminal
/// is resized.
#[derive(Default)]
pub struct SizeAndPosition {
    /// Fixed size adjustment (width, height) in characters
    pub size_offset: (i16, i16),
    /// Fixed position adjustment (x, y) in character coordinates
    pub position_offset: (i16, i16),
    
    /// Size as percentage of terminal area (width%, height%) in the range [0, 1]
    /// The final size is calculated as:
    /// `final_size = (terminal_area * size_area_percent) + size_offset`
    pub size_area_percent: (f32, f32),  // percentage of the terminal area (0.5 is the center)
    /// Position as percentage of terminal area (x%, y%) in the range [0, 1]
    /// The final position is calculated as:
    /// `final_position = (terminal_area * position_area_percent) + position_offset`
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
pub struct StaticWidgetBuilder<C> {
    /// The unique name identifier for the widget.
    name: String,
    /// The z-index depth of the widget; higher values render on top of lower ones.
    depth: Option<u16>,
    /// Whether the widget should have a border.
    border: bool,
    /// The title of the widget, if any.
    title: Option<String>,
    /// The size and position configuration for the widget.
    pub size_and_position: SizeAndPosition,
    /// The custom render function for the widget, if any.
    pub render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,

    __phantom: std::marker::PhantomData<C>,
}

/// Implementations for the methods in `WidgetBuilder`.
impl<C: 'static> WidgetBuilder<C> for StaticWidgetBuilder<C> {
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
    fn build(self, display_area: &crate::render::Rect) -> Result<(Box<dyn Widget<C>>, crate::render::Window), WidgetBuilderError> {
        let (position, size) = self.size_and_position.get_size_and_position(display_area);
        if size.0 == 0 || size.1 == 0 || position.0 == 0 || position.1 == 0 {
            return Err(WidgetBuilderError { details: String::from("Position and/or size cannot be zero when building a new widget or window.") })
        }
        let depth = self.depth.as_ref().unwrap_or(&0u16);
        let mut window = crate::render::Window::new(position, *depth, size);
        if self.border {  window.bordered();  }
        if let Some(title) = &self.title {  window.titled(title.clone());  }
        Ok((Box::new(StaticWidget::<C> {
            children: vec![],
            name: self.name,
            parent_index: None,
            size_and_position: self.size_and_position,
            render_function: self.render_function,
            __phantom: std::marker::PhantomData,
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
        self.border = border;
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
            name,
            depth: None,
            size_and_position: SizeAndPosition::default(),
            render_function: None,
            border: false,
            title: None,
            __phantom: std::marker::PhantomData,
        }
    }
    
    /// Sets the SizeAndPosition configuration directly.
    fn with_sap(mut self, sap: SizeAndPosition) -> Self {
        self.size_and_position = sap;
        self
    }

    /// Static widgets do not respond to events, so this is a no-op that returns self.
    fn with_update_handler(self, _handler: Box<dyn Fn(&mut dyn Widget<C>, &mut C, &mut crate::App<C>, &mut crate::widget::Scene<C>)>) -> Self {
        // static widgets don't need an update handler
        self
    }
}

/// A widget that renders static content using a provided closure (i.e.
/// a title box or description).
/// Suitable for content that doesn't change frequently or in response to events.
/// `StaticWidgetBuilder` is the associated builder for creating instances of this widget.
/// The generic parameter C represents the application data type, which can be any type defined by the user.
pub struct StaticWidget<C> {
    /// The indices of child widgets in the scene graph.
    children: Vec<usize>,

    /// The unique name identifier for the widget. The rendering backend
    /// relies on `String` names instead of widgets.
    name: String,

    /// The index of the parent widget in the scene graph, if any (None would
    /// indicate the root node, which there can only be one of).
    parent_index: Option<usize>,
    
    // this should be an easy, lightweight, and changeable way to get the size and position
    // it needs to be light enough to be changed to something new if the state of the app changes
    // (such as a menu opening and shifting things).
    /// Configuration for the widget's size and position, supporting both static and dynamic layouts.
    pub size_and_position: SizeAndPosition,
    
    // takes the size and position in, and returns the vector of spans to render
    // this is a function object, allowing for capturing of state if desired
    /// Optional closure that generates the widget's rendered content based on size and position.
    pub render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,

    __phantom: std::marker::PhantomData<C>,
}

impl<C> StaticWidget<C> {
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
    ) -> Result<(StaticWidget<C>, crate::render::Window), WidgetErr> {
        let (size, position) = size_and_position.get_size_and_position(display_area);
        if size.0 == 0 || size.1 == 0 || position.0 == 0 || position.1 == 0 {
            return Err(WidgetErr::new("Size or position cannot be zero"))
        }
        let window = crate::render::Window::new(position, depth, size);
        
        let widget = StaticWidget::<C> {
            children: vec![],
            name,
            parent_index: None,
            size_and_position,
            render_function,
            __phantom: std::marker::PhantomData,
        };
        
        Ok((widget, window))
    }
}

/// Implementation of the methods for StaticWidget
impl<C> Widget<C> for StaticWidget<C> {
    /// Returns the widget's name as an identifier. The rendering backend relies
    /// on `String` names instead of widgets.
    fn get_window_ref(&self) -> String {
        self.name.clone()
    }
    
    // for handling updates (a static widget would just have this empty)
    /// Handles event updates (no-op for static widgets as they don't respond to events)
    fn update_with_events(&mut self, _data: &mut C, _app: &mut crate::App<C>, _scene: &mut crate::widget::Scene<C>) {
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


/// Builder for creating DynamicWidget instances with a fluent interface.
/// Maintains configuration state until build() is called to create the actual widget.
/// `DynamicWidgetBuilder` is an example of an implementation of `WidgetBuilder`, where
/// the struct doesn't implement `Widget`.
/// Type C represents the application data type, which can be any type defined by the user.
pub struct DynamicWidgetBuilder<C> {
    /// The unique name identifier for the widget.
    name: String,
    /// The z-index depth of the widget; higher values render on top of lower ones.
    depth: Option<u16>,
    /// Whether the widget should have a border.
    border: bool,
    /// The title of the widget, if any.
    title: Option<String>,
    /// The size and position configuration for the widget.
    pub size_and_position: SizeAndPosition,
    /// The custom render function for the widget, if any.
    pub render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,

    update_handler: Option<Box<dyn Fn(&mut dyn Widget<C>, &mut C, &mut crate::App<C>, &mut crate::widget::Scene<C>)>>,

    __phantom: std::marker::PhantomData<C>,
}

/// Implementations for the methods in `WidgetBuilder`.
impl<C: 'static> WidgetBuilder<C> for DynamicWidgetBuilder<C> {
    /// Constructs a `DynamicWidget`, an implementor of `Widget`, given the parameters.
    /// Validates that size and position are non-zero before creating the widget.
    /// The method takes in a reference to the terminal's current area/size.
    /// # Example:
    /// ```
    /// use term_render::widget_impls::{DynamicWidgetBuilder, WidgetBuilder};
    /// use term_render::render::Rect;
    /// let (widget, window) = DynamicWidgetBuilder::builder(String::new())
    ///     .build(&Rect::default())  // replace &Rect with the actual terminal size (such as `&app.area.read()`)
    ///     .expect("Invalid widget position or size.");
    /// ```
    fn build(self, display_area: &crate::render::Rect) -> Result<(Box<dyn Widget<C>>, crate::render::Window), WidgetBuilderError> {
        let (position, size) = self.size_and_position.get_size_and_position(display_area);
        if size.0 == 0 || size.1 == 0 || position.0 == 0 || position.1 == 0 {
            return Err(WidgetBuilderError { details: String::from("Position and/or size cannot be zero when building a new widget or window.") })
        }
        let depth = self.depth.as_ref().unwrap_or(&0u16);
        let mut window = crate::render::Window::new(position, *depth, size);
        if self.border {  window.bordered();  }
        if let Some(title) = &self.title {  window.titled(title.clone());  }
        Ok((Box::new(DynamicWidget::<C> {
            children: vec![],
            name: self.name,
            parent_index: None,
            size_and_position: self.size_and_position,
            render_function: self.render_function,
            update_handler: self.update_handler,
            __phantom: std::marker::PhantomData,
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
        self.border = border;
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
    /// use term_render::widget_impls::{DynamicWidgetBuilder, WidgetBuilder};
    /// use term_render::render::Rect;
    ///
    /// // the closure can capture local variables to reduce boilerplate
    /// let closure = Box::new(move |size, position| {
    ///     None  // this will leave the widget un-updated
    /// });
    /// let (widget, window) = DynamicWidgetBuilder::builder(String::new())
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
    /// use term_render::widget_impls::{DynamicWidgetBuilder, WidgetBuilder};
    /// let builder = DynamicWidgetBuilder::builder(String::from("Widget Name"));
    /// ```
    fn builder(name: String) -> Self {
        Self {
            name,
            depth: None,
            size_and_position: SizeAndPosition::default(),
            render_function: None,
            border: false,
            title: None,
            update_handler: None,
            __phantom: std::marker::PhantomData,
        }
    }
    
    /// Sets the SizeAndPosition configuration directly.
    fn with_sap(mut self, sap: SizeAndPosition) -> Self {
        self.size_and_position = sap;
        self
    }

    /// Sets the widget's update handler closure. This closure is called during event updates.
    /// The closure receives references to the widget itself, the event parser, and mutable application data.
    /// By default, there is no update handler, meaning the widget won't respond to events.
    fn with_update_handler(mut self, handler: Box<dyn Fn(&mut dyn Widget<C>, &mut C, &mut crate::App<C>, &mut crate::widget::Scene<C>)>) -> Self {
        self.update_handler = Some(handler);
        self
    }
}

/// A widget that renders dynamic content using a provided closure (i.e.
/// a button or interactable component).
/// Suitable for content that changes frequently or in response to events.
/// However, other widget implementations may be more appropriate for complex interactions
/// or multi-button arrangements within a single widget.
/// `DynamicWidgetBuilder` is the associated builder for creating instances of this widget.
pub struct DynamicWidget<C> {
    /// The indices of child widgets in the scene graph.
    children: Vec<usize>,

    /// The unique name identifier for the widget. The rendering backend
    /// relies on `String` names instead of widgets.
    name: String,

    /// The index of the parent widget in the scene graph, if any (None would
    /// indicate the root node, which there can only be one of).
    parent_index: Option<usize>,
    
    // this should be an easy, lightweight, and changeable way to get the size and position
    // it needs to be light enough to be changed to something new if the state of the app changes
    // (such as a menu opening and shifting things).
    /// Configuration for the widget's size and position, supporting both static and dynamic layouts.
    pub size_and_position: SizeAndPosition,
    
    // takes the size and position in, and returns the vector of spans to render
    // this is a function object, allowing for capturing of state if desired
    /// Optional closure that generates the widget's rendered content based on size and position.
    pub render_function: Option<Box<dyn Fn((u16, u16), (u16, u16)) -> Option<Vec<crate::render::Span>>>>,

    /// Optional closure that handles updates to the widget's state.
    pub update_handler: Option<Box<dyn Fn(&mut dyn Widget<C>, &mut C, &mut crate::App<C>, &mut crate::widget::Scene<C>)>>,

    __phantom: std::marker::PhantomData<C>,
}

impl<C> DynamicWidget<C> {
    /// Creates a new DynamicWidget and its associated window.
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
    ) -> Result<(DynamicWidget<C>, crate::render::Window), WidgetErr> {
        let (size, position) = size_and_position.get_size_and_position(display_area);
        if size.0 == 0 || size.1 == 0 || position.0 == 0 || position.1 == 0 {
            return Err(WidgetErr::new("Size or position cannot be zero"))
        }
        let window = crate::render::Window::new(position, depth, size);
        
        let widget = DynamicWidget::<C> {
            children: vec![],
            name,
            parent_index: None,
            size_and_position,
            render_function,
            update_handler: None,
            __phantom: std::marker::PhantomData,
        };
        
        Ok((widget, window))
    }
}

/// Implementation of the methods for DynamicWidget
impl<C> Widget<C> for DynamicWidget<C> {
    /// Returns the widget's name as an identifier. The rendering backend relies
    /// on `String` names instead of widgets.
    fn get_window_ref(&self) -> String {
        self.name.clone()
    }
    
    /// Handles event updates by invoking the user-provided update handler closure, if any.
    /// The closure receives references to the widget itself, the event parser, and mutable application data.
    /// If no update handler is set, this method performs no action.
    fn update_with_events(&mut self, data: &mut C, app: &mut crate::App<C>, scene: &mut crate::widget::Scene<C>) {
        if let Some(update_handler) = self.update_handler.take() {
            update_handler(self, data, app, scene);
            self.update_handler = Some(update_handler);
        }
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



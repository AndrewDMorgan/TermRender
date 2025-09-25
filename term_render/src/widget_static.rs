use crate::widget_impls::*;
use crate::widget::*;

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
    /// let (widget, window) = StaticWidgetBuilder::<AppData>::builder(String::new())
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
    /// let (widget, window) = StaticWidgetBuilder::<AppData>::builder(String::new())
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
    /// let builder = StaticWidgetBuilder::<AppData>::builder(String::from("Widget Name"));
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
    type FunctionType = Box<dyn Fn(&mut dyn Widget<C>, &mut C, &mut crate::App<C>, &mut crate::widget::Scene<C>)>;
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

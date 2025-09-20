#![allow(dead_code)]  // to avoid redundant warnings as this is a library module

// handles widgets and all between
use crate::{event_handler, render as term_render, render, SendSync};

/// A trit representing the internal clockwork for a given widget.
/// Additional functionality can be built on top of this trait.
/// This trait provides the core necessities for a widget to function within the rendering system.
pub trait Widget {
    // a simple way of forcing the user to use an underlying window (necessary for the renderer to work)
    fn get_window_ref(&self) -> String;
    
    // for handling updates (a static widget would just have this empty)
    fn update_with_events(&mut self, events: &SendSync<event_handler::KeyParser>);
    
    /// Updates the underlying window.
    fn update_render(&mut self, window: &mut term_render::Window, area: &term_render::Rect);
    
    fn get_children_indexes(&self) -> Vec<usize>;
    fn add_child_index(&mut self, index: usize);
    fn remove_child_index(&mut self, index: usize);
    fn clear_children_indexes(&mut self);
    
    fn get_parent_index(&self) -> Option<usize>;
    fn set_parent_index(&mut self, index: Option<usize>);
}

/// Represents a generic error related to widget operations.
#[derive(Debug)]
pub struct WidgetErr {
    details: String,
}

impl std::fmt::Display for WidgetErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WidgetErr: {}", self.details)
    }
}

impl WidgetErr {
    /// Creates a new `WidgetErr` with the given details.
    pub fn new(details: &str) -> Self {
        WidgetErr { details: details.to_string() }
    }
}

/// A vector that reserves positions of removed items to maintain stable indices.
/// This is useful in scenarios where the indices of items need to remain consistent,
/// even after some items are removed.
struct PositionReservedVector<T: Widget + ?Sized> {
    pub vector: Vec<Option<Box<T>>>,
    reserved_positions: Vec<usize>,
}

impl <T: ?Sized + Widget> PositionReservedVector<T> {
    /// Removes an item from the vector at the specified index.
    /// The position is then marked as reserved for future use.
    /// This ensures that indices of other items remain unchanged.
    pub fn remove(&mut self, index: usize) -> Result<Box<T>, WidgetErr> {
        if index >= self.vector.len() {
            return Err(WidgetErr::new("Index out of bounds"));
        }
        if self.reserved_positions.contains(&index) {
            return Err(WidgetErr::new("Index is reserved and cannot be removed"));
        }
        
        let item = self.vector.remove(index);
        self.vector.insert(index, None);  // replace with a default value to maintain indices
        self.reserved_positions.push(index);
        
        Ok(item.unwrap_or(Err(WidgetErr::new("Invalid widget index"))?))
    }
    
    /// Pushes an item into the vector, reusing reserved positions if available.
    /// Returns the index where the item was placed.
    pub fn push(&mut self, item: Box<T>) -> usize {
        if let Some(reserved_index) = self.reserved_positions.pop() {
            self.vector[reserved_index] = Some(item);
            reserved_index
        } else {
            self.vector.push(Some(item));
            self.vector.len() - 1
        }
    }
    
    pub fn len(&self) -> usize {
        self.vector.len()
    }
    
    pub fn index(&self, index: usize) -> Option<&Box<T>> {
        if index >= self.vector.len() {
            return None;
        }
        self.vector[index].as_ref()
    }
    
    pub fn index_mut(&mut self, index: usize) -> Option<&mut Box<T>> {
        if index >= self.vector.len() {
            return None;
        }
        self.vector[index].as_mut()
    }
}

pub struct Scene {
    /// All widgets in the scene
    widgets: PositionReservedVector<dyn Widget>,
    /// The hierarchy of widgets (parent-child relationships)
    root_index: Option<usize>,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            widgets: PositionReservedVector {
                vector: Vec::new(),
                reserved_positions: Vec::new(),
            },
            root_index: None,
        }
    }
    
    pub fn widget_as_ref(&self, index: usize) -> Result<&Box<dyn Widget>, WidgetErr> {
        self.widgets.index(index).ok_or(WidgetErr::new("Index out of bounds"))
    }
    
    pub fn widget_as_mut(&mut self, index: usize) -> Result<&mut Box<dyn Widget>, WidgetErr> {
        self.widgets.index_mut(index).ok_or(WidgetErr::new("Index out of bounds"))
    }
    
    // whenever a widget is updated, all its parents need to be updated as well
    pub fn add_widget(&mut self, widget: Box<dyn Widget>, parent: Option<usize>, window: term_render::Window, app: &mut term_render::App) -> Result<usize, WidgetErr> {
        app.add_window(window, widget.get_window_ref(), vec![]);
        
        //let index = self.widgets.len();
        let index = self.widgets.push(widget);
        
        // adding the optional parent-child relationship (only the root node can be parentless)
        if let Some(parent_index) = parent {
            self.widgets.index_mut(parent_index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).add_child_index(index);
            self.widgets.index_mut(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).set_parent_index(parent);
        } else {
            if self.root_index.is_some() {
                return Err(WidgetErr::new("Only one root widget allowed"));
            }
            self.root_index = Some(index);
        }
        
        Ok(index)
    }
    
    pub fn remove_widget(&mut self, index: usize, app: &mut term_render::App) -> Result<(), WidgetErr> {
        if index >= self.widgets.len() {
            return Err(WidgetErr::new("Index out of bounds"));
        }
        
        app.remove_window(self.widgets.index(index).unwrap().get_window_ref()).unwrap();
        
        // updating the parents windows
        self.update_parents(index, app)?;
        
        // remove from parent's children list
        if let Some(parent_index) = self.widgets.index(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).get_parent_index() {
            self.widgets.index_mut(parent_index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).remove_child_index(index);
        } else {
            // if it's the root, clear the root index
            self.root_index = None;
        }
        
        // remove all children recursively
        let children = self.widgets.index(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).get_children_indexes();
        for &child_index in &children {
            self.remove_widget(child_index, app)?;
        }
        
        // finally, remove the widget itself
        self.widgets.remove(index)?;
        
        Ok(())
    }
    
    /// Updates the state of all widgets in the scene.
    /// Also updates the underlying windows of each widget.
    pub fn update_all_widgets(&mut self, events: &SendSync<event_handler::KeyParser>, app: &mut render::App, area: &term_render::Rect) {
        for i in 0..self.widgets.len() {
            if let Some(widget) = self.widgets.index_mut(i) {
                widget.update_with_events(events);
                let window = widget.get_window_ref();
                widget.update_render(app.get_window_reference_mut(window), area);
            }
        }
    }
    
    pub fn force_update_all_widgets(&mut self, app: &mut render::App) {
        for i in 0..self.widgets.len() {
            if let Some(widget) = self.widgets.index_mut(i) {
                //widget.update_render();  // this should already have been done
                app.get_window_reference_mut(widget.get_window_ref()).update_all();
            }
        }
    }
    
    pub fn update_widget(&mut self, index: usize, events: &SendSync<event_handler::KeyParser>, app: &mut term_render::App, area: &term_render::Rect) -> Result<(), WidgetErr> {
        if index >= self.widgets.len() {
            return Err(WidgetErr::new("Index out of bounds"));
        }
        
        let widget = self.widgets.index_mut(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?);
        widget.update_with_events(events);
        let window = app.get_window_reference_mut(widget.get_window_ref());
        widget.update_render(window, area);
        self.update_parents(index, app)?;
        
        Ok(())
    }
    
    pub fn update_widget_renderer(&mut self, index: usize, app: &mut term_render::App, area: &term_render::Rect) -> Result<(), WidgetErr> {
        let widget = self.widgets.index_mut(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?);
        let window = app.get_window_reference_mut(widget.get_window_ref());
        widget.update_render(window, area);
        Ok(())
    }
    
    fn update_parents(&mut self, index: usize, app: &mut term_render::App) -> Result<(), WidgetErr> {
        if let Some(parent_index) = self.widgets.index(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).get_parent_index() {
            let widget = self.widgets.index_mut(parent_index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?);
            app.get_window_reference_mut(widget.get_window_ref()).update_all();
            self.update_parents(parent_index, app)?;
        } Ok(())
    }
}


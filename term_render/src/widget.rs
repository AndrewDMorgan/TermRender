#![allow(dead_code)]  // to avoid redundant warnings as this is a library module

// handles widgets and all between
use crate::{event_handler, render as term_render, render, SendSync};

/// Core trait defining the interface for all UI widgets in the scene graph.
/// Provides methods for event handling, rendering, and managing parent-child relationships.
/// Implementors must provide a window reference and handle updates.
/// The trait represents an abstracted layer above the base Window struct.
pub trait Widget {
    /// Returns a unique identifier string for the widget's associated window.
    /// This connects the widget to its rendering surface in the terminal.
    fn get_window_ref(&self) -> String;
    
    /// Processes input events and updates widget state accordingly.
    /// Static widgets may leave this empty, while interactive widgets should respond to events.
    fn update_with_events(&mut self, events: &SendSync<event_handler::KeyParser>);
    
    /// Updates the widget's visual representation based on current state.
    /// Called automatically during render passes to refresh the terminal display.
    /// Static widgets may leave this empty, while interactive widgets should respond to events.
    /// Returns true if the widget's content changed and needs re-rendering (mainly to indicate
    /// the need for re-rendering the parents).
    fn update_render(&mut self, window: &mut term_render::Window, area: &term_render::Rect) -> bool;
    
    /// Returns indices of child widgets for scene graph traversal.
    fn get_children_indexes(&self) -> Vec<usize>;

    /// Adds a child widget index to maintain parent-child relationships.
    fn add_child_index(&mut self, index: usize);

    /// Removes a child widget index from this widget.
    fn remove_child_index(&mut self, index: usize);

    /// Clears all child widget indices from this widget.
    fn clear_children_indexes(&mut self);
    
    /// Returns the parent widget index if one exists, otherwise returns `None`.
    fn get_parent_index(&self) -> Option<usize>;
    
    /// Sets or clears the parent widget index for hierarchy management.
    fn set_parent_index(&mut self, index: Option<usize>);
}

/// Error type for widget operations, containing descriptive error messages.
/// Used throughout the widget system for consistent error handling.
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
    /// Creates a new `WidgetErr` with the given details. Currently,
    /// this is a very simple struct format, so details will already need
    /// to be coalesced into a single string.
    pub fn new(details: &str) -> Self {
        WidgetErr { details: details.to_string() }
    }
}

/// A specialized vector that maintains stable indices after item removal.
/// Uses a free-list of reserved positions to allow O(1) insertion/removal
/// while preserving indices of existing elements - crucial for UI scene graphs.
struct PositionReservedVector<T: Widget + ?Sized> {
    pub vector: Vec<Option<Box<T>>>,
    /// List of indices that have been removed and can be reused.
    reserved_positions: Vec<usize>,
}

impl <T: ?Sized + Widget> PositionReservedVector<T> {
    /// Removes an item at the given index, replacing it with `None`, leaving the indices intact.
    /// Marks the position as reserved for future reuse.
    /// Returns the removed item or an error if index is invalid.
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
    
    /// Returns the number of items in the vector (including reserved positions).
    pub fn len(&self) -> usize {
        self.vector.len()
    }

    /// Returns the number of actual items (excluding reserved positions).
    pub fn adjusted_len(&self) -> usize {
        self.vector.len() - self.reserved_positions.len()
    }
    
    /// Returns a reference to the item at the given index, if it exists.
    /// If the index is out of bounds *or* reserved, returns None.
    pub fn index(&self, index: usize) -> Option<&Box<T>> {
        if index >= self.vector.len() {
            return None;
        }
        self.vector[index].as_ref()
    }
    
    /// Returns a mutable reference to the item at the given index, if it exists.
    /// If the index is out of bounds *or* reserved, returns None.
    pub fn index_mut(&mut self, index: usize) -> Option<&mut Box<T>> {
        if index >= self.vector.len() {
            return None;
        }
        self.vector[index].as_mut()
    }
}

/// Manages a collection of widgets and their hierarchical relationships.
/// Handles rendering coordination, event propagation, and widget lifecycle.
pub struct Scene {
    /// All widgets in the scene
    widgets: PositionReservedVector<dyn Widget>,
    /// The hierarchy of widgets (parent-child relationships)
    root_index: Option<usize>,
}

impl Scene {
    /// Creates a new empty scene with no widgets.
    pub fn new() -> Self {
        Scene {
            widgets: PositionReservedVector {
                vector: Vec::new(),
                reserved_positions: Vec::new(),
            },
            root_index: None,
        }
    }
    
    /// Returns a reference to the widget at the given index.
    /// Returns an error if the index is out of bounds.
    pub fn widget_as_ref(&self, index: usize) -> Result<&Box<dyn Widget>, WidgetErr> {
        self.widgets.index(index).ok_or(WidgetErr::new("Index out of bounds"))
    }
    
    /// Returns a mutable reference to the widget at the given index.
    /// Returns an error if the index is out of bounds.
    pub fn widget_as_mut(&mut self, index: usize) -> Result<&mut Box<dyn Widget>, WidgetErr> {
        self.widgets.index_mut(index).ok_or(WidgetErr::new("Index out of bounds"))
    }
    
    // whenever a widget is updated, all its parents need to be updated as well
    /// Adds a widget to the scene and registers its window with the renderer.
    /// Establishes parent-child relationships and handles root node assignment.
    /// Returns the index where the widget was placed.
    pub fn add_widget(&mut self, widget: Box<dyn Widget>, window: term_render::Window, app: &mut term_render::App) -> Result<usize, WidgetErr> {
        app.add_window(window, widget.get_window_ref(), vec![]);
        
        //let index = self.widgets.len();
        let parent_index = widget.get_parent_index();
        let index = self.widgets.push(widget);
        
        // adding the optional parent-child relationship (only the root node can be parentless)
        if let Some(parent_index) = &parent_index {
            self.widgets.index_mut(*parent_index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).add_child_index(index);
            self.widgets.index_mut(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).set_parent_index(Some(*parent_index));
        } else {
            if self.root_index.is_some() {
                return Err(WidgetErr::new("Only one root widget allowed"));
            }
            self.root_index = Some(index);
        }
        
        Ok(index)
    }
    
    /// Removes a widget and all its children recursively.
    /// Handles window cleanup, parent relationship updates, and resource management.
    /// If a parent is removed, all its children are also removed.
    /// Returns an error if the index is out of bounds or reserved (look at
    /// `PositionReservedVector::reserved_positions` for more information on reservations).
    pub fn remove_widget(&mut self, index: usize, app: &mut term_render::App) -> Result<(), WidgetErr> {
        // checking if it's out of range, or a reserved index
        if index >= self.widgets.len() || self.widgets.index(index).is_none() {
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
    
    /// Updates all widgets in the scene with current events and refreshes their rendering.
    /// Processes events first, then updates visual representation for each widget.
    /// If a widget's content changes, its parents are also updated to reflect the change.
    /// This ensures the entire scene graph remains consistent and up-to-date.
    pub fn update_all_widgets(&mut self, events: &SendSync<event_handler::KeyParser>, app: &mut render::App, area: &term_render::Rect) -> Result<(), WidgetErr> {
        for i in 0..self.widgets.len() {  // the if let skips reserved indices
            if let Some(widget) = self.widgets.index_mut(i) {
                widget.update_with_events(events);
                let window = widget.get_window_ref();
                if widget.update_render(app.get_window_reference_mut(window), area) {
                    // if the widget changed, update all its parents
                    self.update_parents(i, app)?;
                }
            }
        } Ok(())
    }
    
    /// Forces a complete update of all widget windows regardless of dirty state.
    /// Useful when the terminal is resized or major layout changes occur (although,
    /// in most cases, it shouldn't be necessary, and should automatically be handled).
    pub fn force_update_all_widgets(&mut self, app: &mut render::App) {
        for i in 0..self.widgets.len() {
            if let Some(widget) = self.widgets.index_mut(i) {
                //widget.update_render();  // this should already have been done
                app.get_window_reference_mut(widget.get_window_ref()).update_all();
            }
        }
    }
    
    /// Updates a specific widget and its rendering.
    /// Also triggers updates to parent widgets to maintain consistency if the window is updated.
    pub fn update_widget(&mut self, index: usize, events: &SendSync<event_handler::KeyParser>, app: &mut term_render::App, area: &term_render::Rect) -> Result<(), WidgetErr> {
        if index >= self.widgets.len() || self.widgets.index(index).is_none() {
            return Err(WidgetErr::new("Index out of bounds"));
        }
        
        let widget = self.widgets.index_mut(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?);
        widget.update_with_events(events);
        let window = app.get_window_reference_mut(widget.get_window_ref());
        if widget.update_render(window, area) {
            self.update_parents(index, app)?;
        }
        
        Ok(())
    }
    
    /// Updates only the rendering of a specific widget without processing events.
    /// Useful for visual-only changes that don't affect widget state.
    pub fn update_widget_renderer(&mut self, index: usize, app: &mut term_render::App, area: &term_render::Rect) -> Result<(), WidgetErr> {
        let widget = self.widgets.index_mut(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?);
        let window = app.get_window_reference_mut(widget.get_window_ref());
        if widget.update_render(window, area) {
            self.update_parents(index, app)?;
        }
        Ok(())
    }
    
    /// Recursively updates all parent widgets of the widget at the given index.
    /// Ensures visual consistency when child widgets change.
    fn update_parents(&mut self, index: usize, app: &mut term_render::App) -> Result<(), WidgetErr> {
        if let Some(parent_index) = self.widgets.index(index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?).get_parent_index() {
            let widget = self.widgets.index_mut(parent_index).unwrap_or(Err(WidgetErr::new("Invalid widget index"))?);
            app.get_window_reference_mut(widget.get_window_ref()).update_all();
            self.update_parents(parent_index, app)?;
        } Ok(())
    }
}


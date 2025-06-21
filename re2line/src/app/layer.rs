use std::ops::Index;

use super::config::Config;
use super::game::GameObject;

pub struct Layer<O: GameObject>(Vec<O>);

impl<O: GameObject> Layer<O> {
    pub const fn new() -> Self {
        Self(Vec::new())
    }
    
    pub fn objects(&self) -> &[O] {
        self.0.as_slice()
    }
    
    pub fn set_objects(&mut self, objects: Vec<O>) {
        self.0 = objects;
    }
    
    pub fn clear(&mut self) {
        self.0.clear();   
    }
    
    pub fn visible_objects(&self, config: &Config) -> impl Iterator<Item = (usize, &'_ O)> {
        self.0.iter().enumerate().filter(|(_, obj)| config.should_show(obj.object_type()))
    }

    pub fn visible_objects_desc(&self, config: &Config) -> impl Iterator<Item = (usize, &'_ O)> {
        self.0.iter().enumerate().rev().filter(|(_, obj)| config.should_show(obj.object_type()))
    }
    
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl<O: GameObject> Index<usize> for Layer<O> {
    type Output = O;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }   
}
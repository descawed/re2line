use std::ops::Index;

use super::config::Config;
use super::game::GameObject;

pub struct Layer<O: GameObject> {
    name: String,
    objects: Vec<O>,
}

impl<O: GameObject> Layer<O> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            objects: Vec::new(),
        }
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn objects(&self) -> &[O] {
        self.objects.as_slice()
    }
    
    pub fn set_objects(&mut self, objects: Vec<O>) {
        self.objects = objects;
    }
    
    pub fn clear(&mut self) {
        self.objects.clear();   
    }
    
    pub fn visible_objects(&self, config: &Config) -> impl Iterator<Item = (usize, &'_ O)> {
        self.objects.iter().enumerate().filter(|(_, obj)| config.should_show(obj.object_type()))
    }

    pub fn visible_objects_desc(&self, config: &Config) -> impl Iterator<Item = (usize, &'_ O)> {
        self.objects.iter().enumerate().rev().filter(|(_, obj)| config.should_show(obj.object_type()))
    }
    
    pub const fn len(&self) -> usize {
        self.objects.len()
    }
}

impl<O: GameObject> Index<usize> for Layer<O> {
    type Output = O;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.objects[index]
    }   
}
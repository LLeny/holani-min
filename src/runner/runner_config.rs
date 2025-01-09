use std::{collections::HashMap, path::PathBuf};

use macroquad::input::KeyCode;

#[derive(Clone)]
pub(crate) enum Input {
    Up,
    Down,
    Left,
    Right,
    Outside,
    Inside,
    Option1,
    Option2,
    Pause,
}

#[derive(Clone)]
pub(crate) struct RunnerConfig {
    rom: Option<PathBuf>,
    cartridge: Option<PathBuf>,
    button_mapping: HashMap<KeyCode, Input>,
    linear_filter: bool,
    mute: bool,
    comlynx_port: u16,
}

impl RunnerConfig {
    pub(crate) fn new() -> Self {
        Self {
            rom: None,
            cartridge: None,
            linear_filter: false,
            mute: false,
            comlynx_port: 0,
            button_mapping: HashMap::new()
        }
    }

    pub(crate) fn rom(&self) -> &Option<PathBuf> {
        &self.rom
    }

    pub(crate) fn set_rom(&mut self, rom: PathBuf) {
        self.rom = Some(rom);
    }

    pub(crate) fn cartridge(&self) -> &Option<PathBuf> {
        &self.cartridge
    }

    pub(crate) fn set_cartridge(&mut self, cartridge: PathBuf) {
        self.cartridge = Some(cartridge);
    }

    pub(crate) fn button_mapping(&self) -> &HashMap<KeyCode, Input> {
        &self.button_mapping
    }

    pub(crate) fn set_button_mapping(&mut self, key: KeyCode, btn: Input) {
        if let Some(x) = self.button_mapping.get_mut(&key) {
            *x = btn;
        } else {
            self.button_mapping.insert(key, btn);
        }
    }
        
    pub(crate) fn linear_filter(&self) -> bool {
        self.linear_filter
    }
    
    pub(crate) fn set_linear_filter(&mut self, linear_filter: bool) {
        self.linear_filter = linear_filter;
    }
    
    pub(crate) fn mute(&self) -> bool {
        self.mute
    }
    
    pub(crate) fn set_mute(&mut self, mute: bool) {
        self.mute = mute;
    }
    
    pub(crate) fn comlynx_port(&self) -> u16 {
        self.comlynx_port
    }
    
    pub(crate) fn set_comlynx_port(&mut self, port: u16) {
        self.comlynx_port = port;
    }
}

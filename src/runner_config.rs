use std::{collections::HashMap, path::PathBuf};

use holani::suzy::registers::Joystick;
use macroquad::input::KeyCode;

#[derive(Clone)]
pub(crate) struct RunnerConfig {
    rom: Option<PathBuf>,
    cartridge: Option<PathBuf>,
    button_mapping: HashMap<KeyCode, Joystick>,
    linear_filter: bool,
    mute: bool,
}

impl RunnerConfig {
    pub(crate) fn new() -> Self {
        Self {
            rom: None,
            cartridge: None,
            linear_filter: false,
            mute: false,
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

    pub(crate) fn button_mapping(&self) -> &HashMap<KeyCode, Joystick> {
        &self.button_mapping
    }

    pub(crate) fn set_button_mapping(&mut self, key: KeyCode, btn: Joystick) {
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
}

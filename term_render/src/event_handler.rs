#![allow(dead_code)]

use std::io::Write;
use vte::Perform;

// constants for tracking mouse scrolling
const SCROLL_SENSITIVITY: f64 = 0.05;
const SCROLL_LOG_TIME: f64 = 0.75;

#[derive(PartialEq, Eq, Debug, Default)]
pub enum KeyModifiers {
    Shift,
    #[default] Command,
    Option,
    Control,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum KeyCode {
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Return,
    Escape,
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub enum MouseEventType {
    #[default] Null,
    Left,
    Right,
    Middle,
    Down,
    Up,
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub enum MouseState {
    Release,
    Press,
    Hold,
    #[default] Null,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct MouseEvent {
    pub event_type: MouseEventType,
    pub position: (u16, u16),
    pub state: MouseState,
}

#[derive(Default)]
pub struct KeyParser {
    pub key_modifiers: Vec <KeyModifiers>,
    pub key_events: std::collections::HashMap <KeyCode, bool>,
    pub char_events: Vec <char>,
    pub in_escape_seq: bool,
    pub bytes: usize,
    pub mouse_event: Option <MouseEvent>,
    pub mouse_modifiers: Vec <KeyModifiers>,
    pub last_press: u128,
    pub scroll_events: Vec <(std::time::SystemTime, i8)>,  // the sign is the direction
    pub scroll_accumulate: f64,
}

impl KeyParser {
    pub fn new () -> Self {
        KeyParser {
            key_events: std::collections::HashMap::from([
                (KeyCode::Delete, false),
                (KeyCode::Tab, false),
                (KeyCode::Left, false),
                (KeyCode::Right, false),
                (KeyCode::Up, false),
                (KeyCode::Down, false),
                (KeyCode::Return, false),
                (KeyCode::Escape, false),
            ]),
            key_modifiers: vec!(),
            char_events: vec!(),
            in_escape_seq: false,
            bytes: 0,
            mouse_event: None,
            mouse_modifiers: vec!(),
            last_press: 0,
            scroll_events: vec![],
            scroll_accumulate: 0.0,
        }
    }

    // tracking a log of scroll events to average them out over a duration of time
    fn scroll (&mut self, sign: i8) {
        let time = std::time::SystemTime::now();
        if self.scroll_accumulate.is_sign_negative() != sign.is_negative(){
            self.scroll_events.clear();  // so on sign flip it doesn't do weird things
        }
        self.scroll_events.push((time, sign));
        self.update_scroll();
    }

    fn update_scroll(&mut self) {
        let time = std::time::SystemTime::now();
        let mut valid = vec![];
        let mut avg = 0.0;
        for (other_time, other_sign) in &self.scroll_events {
            // 0.000001 is the conversion rate from micro seconds to seconds
            let duration = time.duration_since(*other_time).unwrap_or_default().as_secs_f64();
            if duration < SCROLL_LOG_TIME {
                avg += *other_sign as f64; valid.push((*other_time, *other_sign));
            }
        }
        avg *= SCROLL_SENSITIVITY / SCROLL_LOG_TIME;
        self.scroll_accumulate = avg;
        self.scroll_events = valid;
    }

    pub fn clear_events (&mut self) {
        self.char_events.clear();
        self.key_modifiers.clear();
        self.mouse_modifiers.clear();
        self.key_events.clear();
        self.in_escape_seq = false;
        self.update_scroll();

        if let Some(event) = &mut self.mouse_event {
            match event.state {
                MouseState::Press => {
                    event.state = MouseState::Hold;
                },
                MouseState::Hold if matches!(event.event_type, MouseEventType::Down | MouseEventType::Up) => {
                    event.state = MouseState::Release;
                },
                MouseState::Release => {
                    event.state = MouseState::Null;
                    event.event_type = MouseEventType::Null;
                },
                MouseState::Hold => {
                },
                _ => {},
            }
        }
    }

    pub fn contains_char (&self, chr: char) -> bool {
        self.char_events.contains(&chr)
    }

    pub fn contains_modifier (&self, modifier: &KeyModifiers) -> bool {
        self.key_modifiers.contains(modifier)
    }

    pub fn contains_mouse_modifier (&self, modifier: KeyModifiers) -> bool {
        self.mouse_modifiers.contains(&modifier)
    }

    pub fn contains_key_code (&self, key: KeyCode) -> bool {
        *self.key_events.get(&key).unwrap_or(&false)
    }

    fn handle_mouse_escape_codes (&mut self, numbers: &[u16], c: char) {
        if let Some([byte, x, y]) = numbers.get(0..3) {
            let button = byte & 0b11; // Mask lowest 2 bits (button type)
            //println!("button: {}, numbers: {:?}", button, numbers);

            // adding key press modifiers
            if (byte & 32) != 0 {
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            if (byte & 64) != 0 {
                self.key_modifiers.push(KeyModifiers::Option);
            }
            if (byte & 128) != 0 {
                self.key_modifiers.push(KeyModifiers::Control);
            }

            //println!("Code: {:?} / {}", numbers, c);

            let is_scroll = (byte & 64) != 0;
            let event_type = match (is_scroll, button) {
                (true, 0) => {
                    self.scroll(-1i8);
                    MouseEventType::Up
                },
                (true, 1) => {
                    self.scroll(1i8);
                    MouseEventType::Down
                },
                (false, 0) => MouseEventType::Left,
                (false, 1) => MouseEventType::Middle,
                (false, 2) => MouseEventType::Right,
                _ => MouseEventType::Null
            };

            if matches!(event_type, MouseEventType::Left) && numbers[0] == 4 {
                self.mouse_modifiers.push(KeyModifiers::Shift);
            }

            self.calculate_mouse_event_code(event_type, (*x, *y), c);
        }
    }

    fn calculate_mouse_event_code (
        &mut self,
        event_type: MouseEventType,
        (x, y): (u16, u16),
        c: char
    ) {
        if let Some(event) = &mut self.mouse_event {
            if matches!(event_type, MouseEventType::Left) &&
                event.position != (x, y) &&
                matches!(event.state, MouseState::Hold) &&
                c == 'M'
            {
                event.position = (x, y);
                return;
            }
        }

        self.mouse_event = Some(MouseEvent {
            event_type,
            position: (x, y),
            state: {
                match c {
                    'M' => MouseState::Press,
                    'm' => MouseState::Release,
                    _ => MouseState::Null,
                }
            },
        });
    }

    fn handle_custom_escape_codes (&mut self, numbers: &[u16]) {
        match numbers[1] {
            2 => {
                self.key_events.insert(KeyCode::Delete, true);
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            3 => {
                self.key_events.insert(KeyCode::Delete, true);
                self.key_modifiers.push(KeyModifiers::Option);
            }
            4 => {
                self.key_events.insert(KeyCode::Left, true);
                self.key_modifiers.push(KeyModifiers::Command);
            }
            5 => {
                self.key_events.insert(KeyCode::Right, true);
                self.key_modifiers.push(KeyModifiers::Command);
            }
            6 => {
                self.key_events.insert(KeyCode::Up, true);
                self.key_modifiers.push(KeyModifiers::Command);
            }
            7 => {
                self.key_events.insert(KeyCode::Down, true);
                self.key_modifiers.push(KeyModifiers::Command);
            }
            8 => {
                self.key_events.insert(KeyCode::Delete, true);
                self.key_modifiers.push(KeyModifiers::Option);
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            9 => {
                self.key_events.insert(KeyCode::Delete, true);
                self.key_modifiers.push(KeyModifiers::Command);
            }
            10 => {
                self.key_events.insert(KeyCode::Delete, true);
                self.key_modifiers.push(KeyModifiers::Command);
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            11 => {
                self.key_modifiers.push(KeyModifiers::Command);
                self.char_events.push('s');  // command + s
            }
            12 => {  // lrud
                self.key_events.insert(KeyCode::Left, true);
                self.key_modifiers.push(KeyModifiers::Command);
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            13 => {
                self.key_events.insert(KeyCode::Right, true);
                self.key_modifiers.push(KeyModifiers::Command);
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            14 => {
                self.key_events.insert(KeyCode::Up, true);
                self.key_modifiers.push(KeyModifiers::Command);
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            15 => {
                self.key_events.insert(KeyCode::Down, true);
                self.key_modifiers.push(KeyModifiers::Command);
                self.key_modifiers.push(KeyModifiers::Shift);
            }
            16 => {
                self.key_modifiers.push(KeyModifiers::Command);
                self.char_events.push('c');
            }
            17 => {
                self.key_modifiers.push(KeyModifiers::Command);
                self.char_events.push('v');
            }
            18 => {
                self.key_modifiers.push(KeyModifiers::Command);
                self.char_events.push('x');
            }
            19 => {
                self.key_modifiers.push(KeyModifiers::Command);
                self.char_events.push('f');
            }
            20 => {
                self.key_modifiers.push(KeyModifiers::Command);
                self.char_events.push('z');
            }
            21 => {
                self.key_modifiers.push(KeyModifiers::Command);
                self.key_modifiers.push(KeyModifiers::Shift);
                self.char_events.push('z');
            }
            22 => {
                self.key_events.insert(KeyCode::Tab, true);
                self.key_modifiers.push(KeyModifiers::Option);
            }
            _ => {}
        }
    }

    fn handle_control_arrows (&mut self, _numbers: &[u16], c: char) {
        match c {
            'D' => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.key_events.insert(KeyCode::Left, true);
            },
            'C' => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.key_events.insert(KeyCode::Right, true);
            },
            'A' => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.key_events.insert(KeyCode::Up, true);
            },
            'B' => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.key_events.insert(KeyCode::Down, true);
            },
            _ => {}  // control + arrows
        }
    }

    fn handle_standard_escape_codes (&mut self, numbers: &Vec <u16>, c: char) {
        match c as u8 {
            0x5A => {
                self.key_events.insert(KeyCode::Tab, true);
                self.key_modifiers.push(KeyModifiers::Shift);
            },
            0x44 => {
                self.key_events.insert(KeyCode::Left, true);
                if *numbers == [1, 3] {
                    self.key_modifiers.push(KeyModifiers::Option);
                } else if *numbers == [1, 2] {
                    self.key_modifiers.push(KeyModifiers::Shift);
                } else if *numbers == [1, 4] {
                    self.key_modifiers.push(KeyModifiers::Option);
                    self.key_modifiers.push(KeyModifiers::Shift);
                }
            },
            0x43 => {
                self.key_events.insert(KeyCode::Right, true);
                if *numbers == [1, 3] {
                    self.key_modifiers.push(KeyModifiers::Option);
                } else if *numbers == [1, 2] {
                    self.key_modifiers.push(KeyModifiers::Shift);
                } else if *numbers == [1, 4] {
                    self.key_modifiers.push(KeyModifiers::Option);
                    self.key_modifiers.push(KeyModifiers::Shift);
                }
            },
            0x41 => {
                self.key_events.insert(KeyCode::Up, true);
                if *numbers == [1, 3] {
                    self.key_modifiers.push(KeyModifiers::Option);
                } else if *numbers == [1, 2] {
                    self.key_modifiers.push(KeyModifiers::Shift);
                } else if *numbers == [1, 4] {
                    self.key_modifiers.push(KeyModifiers::Option);
                    self.key_modifiers.push(KeyModifiers::Shift);
                }
            },
            0x42 => {
                self.key_events.insert(KeyCode::Down, true);
                if *numbers == [1, 3] {
                    self.key_modifiers.push(KeyModifiers::Option);
                } else if *numbers == [1, 2] {
                    self.key_modifiers.push(KeyModifiers::Shift);
                } else if *numbers == [1, 4] {
                    self.key_modifiers.push(KeyModifiers::Option);
                    self.key_modifiers.push(KeyModifiers::Shift);
                }
            },
            _ => {},
        }
    }

}

pub fn enable_mouse_capture() {
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"echo -e \"\x1B[?1006h\"");
    let _ = stdout.write_all(b"\x1B[?1000h"); // Enable basic mouse mode
    let _ = stdout.write_all(b"\x1B[?1003h"); // Enable all motion events
}

pub fn disable_mouse_capture() {
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x1B[?1000l"); // Disable mouse mode
    let _ = stdout.write_all(b"\x1B[?1003l"); // Disable motion events
}

impl KeyParser {
    pub fn set_press_time (&mut self) {
        self.last_press = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("Time went backwards...")
            .as_millis();
    }
}

impl Perform for KeyParser {
    fn print(&mut self, chr: char) {
        //println!("char {}: '{}'", chr as u8, chr);
        if self.in_escape_seq || self.bytes > 1 {
            match chr as u8 {
                17 => {
                    self.char_events.push('w');
                    self.key_modifiers.push(KeyModifiers::Option);
                },
                _ => {}
            }

            return;
        }
        self.set_press_time();

        if chr as u8 == 0x7F {
            self.key_events.insert(KeyCode::Delete, true);
            return;
        }
        if !(chr.is_ascii_graphic() || chr.is_whitespace()) {  return;  }
        //println!("char {}: '{}'", chr as u8, chr);
        self.char_events.push(chr);
    }

    #[inline(always)]
    fn execute(&mut self, byte: u8) {
        self.set_press_time();

        // control + ...
        // 3 = c; 22 = v; 26 = z; 6 = f; 1 = a; 24 = x; 19 = s; 21 = u; r = 18
        // left ^[[1;5D right ^[[1;5C up ^[[1;5A down ^[[1;5B
        // control u and control r and necessary for undo and redo bc/
        // control + key and control + shift + key don't send unique
        // escape codes for some odd reason

        match byte {
            0x1B => {
                self.in_escape_seq = true;
            },
            0x0D => {  // return aka \n
                self.key_events.insert(KeyCode::Return, true);
            },
            0x09 => {
                self.key_events.insert(KeyCode::Tab, true);
            },// 3 = c; 22 = v; 26 = z; 6 = f; 1 = a; 24 = x; 19 = s; 21 = u; r = 18
            3 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('c');
            },
            22 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('v');
            },
            26 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('z');
            },
            6 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('f');
            },
            1 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('a');
            },
            24 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('x');
            },
            19 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('s');
            },
            21 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('u');
            },
            18 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.char_events.push('r');
            },
            0x08 => {
                self.key_modifiers.push(KeyModifiers::Control);
                self.key_events.insert(KeyCode::Delete, true);
            },
            10 => {
                self.char_events.push('a');
                self.key_modifiers.push(KeyModifiers::Control);
            },
            _ => {},
        }
        //println!("byte {}: '{}'", byte, byte as char);
    }

    #[inline(always)]
    fn csi_dispatch(&mut self, params: &vte::Params, _: &[u8], _: bool, c: char) {
        self.in_escape_seq = false;  // resetting the escape sequence
        self.set_press_time();

        let numbers: Vec <u16> = params.iter().map(|p| p[0]).collect();

        // mouse handling
        if c == 'M' || c == 'm' {
            self.handle_mouse_escape_codes(&numbers, c);
            return;
        }

        //for number in &numbers {println!("{}", number);}
        if c == '~' && numbers.len() == 2 && numbers[0] == 3 {  // this section is for custom escape codes
            self.handle_custom_escape_codes(&numbers);
        } else if numbers.len() == 2 && numbers[0] == 1 && numbers[1] == 5 {
            // control + ...
            // 3 = c; 22 = v; 26 = z; 6 = f; 1 = a; 24 = x; 19 = s; 21 = u; r = 18
            // left ^[[1;5D right ^[[1;5C up ^[[1;5A down ^[[1;5B
            // control u and control r and necessary for undo and redo bc/
            // control + key and control + shift + key don't send unique
            // escape codes for some odd reason
            self.handle_control_arrows(&numbers, c);
        } else {  // this checks existing escape codes of 1 parameter/ending code (they don't end with ~)
            self.handle_standard_escape_codes(&numbers, c);
        }
    }
}


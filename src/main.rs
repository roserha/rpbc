use core::f64;
use std::env;
use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use json::JsonValue;
use pitch_detector::{
    pitch::{HannedFftDetector, PitchDetector},
};
use std::fs::File;
use std::io::{Read, Write};
use console::{Style, Term};
use rdev::{simulate, Button, Event, EventType, Key, SimulateError};
use mouse_position::mouse_position::{Mouse};

// Bunch of global increments
static mut GLOBAL_INCREMENT_A:u32 = 0;
static mut GLOBAL_INCREMENT_B:usize = 0;

// Bunch of global nums
static mut GLOBAL_FLOAT_A:f64 = 0.0;
static mut GLOBAL_INT_A:usize = 0;

// Bunch of global bools
static mut GLOBAL_BOOL_A:bool = false;
static mut GLOBAL_BOOL_B:bool = false;
static mut CONTINUE_TO_RUN_PROGRAM:bool = true;

// Terminal shenanigans
static mut TERM:Option<Term> = None;

// Buttons and pitches
const BUTTONS:&[&str] = &["A", "B", "Select", "Start", "→", "←", "↑", "↓", "R", "L", "X", "Y", "Top", "Left", "Bottom", "Right"];
const KEYS:&[Key] = &[Key::KeyJ, Key::KeyN, Key::ShiftLeft, Key::Return, Key::KeyD, Key::KeyA, Key::KeyW, Key::KeyS, Key::KeyR, Key::KeyL, Key::KeyU, Key::KeyH];
static mut PITCHES:Vec<f64> = Vec::new();
static mut MOUSECOORDS:Vec<i32> = Vec::new();

// Flags
enum FlagValues {
    Calibrating = 1,
    HideInputDevices = 2,
    SeeButtonFrequency = 4,
    SkipPitch = 8,
    DisableControl = 16,
}

// Core part of program, where funky things are done with the "raw" pitch itself
fn pitch_functionality(l_pitch:&f64, r_pitch:&f64, flags:&u128) {
    unsafe {
        let avg_pitch = (l_pitch + r_pitch) / 2.0;
        let mut terminal = TERM.clone().unwrap();
        terminal.clear_line().unwrap();
        terminal.move_cursor_up(1).unwrap();
        terminal.clear_line().unwrap();

        if avg_pitch < 50.0 { // Not pressing anything
            terminal.write(b"No buttons.. [ ]\nNo touch screen either...").unwrap();
            GLOBAL_BOOL_A = false;
            if GLOBAL_INT_A == 200 {
                if (flags & FlagValues::DisableControl as u128 == 0) {
                    let event = EventType::ButtonRelease(Button::Left);
                    simulate(&event).unwrap();
                }
                GLOBAL_INT_A = 0;
            } else if GLOBAL_INT_A > 0 {
                if (flags & FlagValues::DisableControl as u128 == 0) {
                    let event = EventType::KeyRelease(*(KEYS.get(GLOBAL_INT_A - 1).unwrap()));
                    simulate(&event).unwrap();
                }
                GLOBAL_INT_A = 0;
                GLOBAL_INCREMENT_A = 0;
            }
        } 

        // Pressing buttons
        else if avg_pitch < 525.0 { 
            let mut difference_in_pitch:f64 = f64::INFINITY;
            let mut i = 0;
            for j in 0..13 {
                let new_difference_in_pitch = f64::abs(avg_pitch - PITCHES.get(j).unwrap());
                // println!("{}|{}: {}", i, j, new_difference_in_pitch);
                if new_difference_in_pitch > difference_in_pitch {
                    i = j - 1;
                    break;
                } else {
                    difference_in_pitch = new_difference_in_pitch;
                }
            }
            terminal.write(b"Pressing the [").unwrap();
            let button_style = Style::new().cyan().bold();
            print!("{}", button_style.apply_to(BUTTONS.get(i).unwrap()));
            terminal.write(b"] button!").unwrap();
            if flags & FlagValues::SeeButtonFrequency as u128 != 0 {
                print!(" || ({} Hz", button_style.apply_to(avg_pitch));
                terminal.write(b")").unwrap();
            }
            terminal.write(b"\nNo touch screen though...").unwrap();

            if !GLOBAL_BOOL_A {
                if GLOBAL_INCREMENT_A > 2 {
                    GLOBAL_INT_A = i + 1;
                    if (flags & FlagValues::DisableControl as u128 == 0) {
                        let event = EventType::KeyPress(*(KEYS.get(i).unwrap()));
                        simulate(&event);
                    }
                    GLOBAL_BOOL_A = true;
                } else {GLOBAL_INCREMENT_A += 1;}
            }
        } 

        // Pressing touch screen -> L: Horizontal || R: Vertical
        
        else { 
            terminal.write(b"No buttons.. [ ]\nBut screen coords (").unwrap();
            let x_range = PITCHES.get(15).unwrap() - PITCHES.get(13).unwrap();
            let y_range = PITCHES.get(14).unwrap() - PITCHES.get(12).unwrap();
            let x_coord = f64::max(0.0, f64::min(1.0,(l_pitch - PITCHES.get(13).unwrap()) / x_range));
            let y_coord = f64::max(0.0, f64::min(1.0,(r_pitch - PITCHES.get(12).unwrap()) / y_range));
            let x_style = Style::new().magenta().bold();
            let y_style = Style::new().yellow().bold();
            print!("{:.3}, {:.3}", x_style.apply_to(x_coord), y_style.apply_to(y_coord));
            terminal.write(b")").unwrap();
            if flags & FlagValues::SeeButtonFrequency as u128 != 0 {
                print!(" || [{} Hz, {} Hz", x_style.apply_to(l_pitch), y_style.apply_to(r_pitch));
                terminal.write(b")").unwrap();
            }

            let mouse_x_coord = (*MOUSECOORDS.get(0).unwrap()) as f64 + (x_coord * (MOUSECOORDS.get(2).unwrap() - MOUSECOORDS.get(0).unwrap()) as f64);
            let mouse_y_coord = (*MOUSECOORDS.get(1).unwrap()) as f64 + (y_coord * (MOUSECOORDS.get(3).unwrap() - MOUSECOORDS.get(1).unwrap()) as f64);

            if (flags & FlagValues::DisableControl as u128 == 0) {
                let event = EventType::MouseMove { x: mouse_x_coord, y: mouse_y_coord };
                simulate(&event).unwrap();
            }
            if !GLOBAL_BOOL_A {
                GLOBAL_INT_A = 200;
                if (flags & FlagValues::DisableControl as u128 == 0) {
                    let event = EventType::ButtonPress(Button::Left);
                    simulate(&event).unwrap();
                }
                GLOBAL_BOOL_A = true;
            }
        }
    }
}

// Calibrating the pitches for the things to be recognized
fn calibration_pitch_functionality(l_pitch:&f64, r_pitch:&f64, flags:&u128) {
    unsafe{
        // a = how many samples used
        // b = which sample using

        if flags & FlagValues::SkipPitch as u128 != 0 && !GLOBAL_BOOL_B {
            GLOBAL_BOOL_B = true;
            GLOBAL_INCREMENT_B = 16;
        }

        let avg_pitch = (l_pitch + r_pitch) / 2.0;

        // Calibrate Buttons
        if GLOBAL_INCREMENT_B < 12 {
            if avg_pitch > 50.0 { // if holding button
                if GLOBAL_INCREMENT_A >= 200 { // if already enough samples
                    if GLOBAL_BOOL_A {
                        PITCHES.push(GLOBAL_FLOAT_A);
                        let mut terminal = TERM.clone().unwrap();
                        terminal.clear_line().unwrap();
                        print!("    Frequency: {:.2}Hz. To continue, let go of button.\n", GLOBAL_FLOAT_A);
                        GLOBAL_BOOL_A = false;
                    }
                } else {
                    let mut terminal = TERM.clone().unwrap();
                    terminal.clear_line().unwrap();
                    terminal.write(b"Calibrating, ").unwrap();
                    print!("{:.2}", (GLOBAL_INCREMENT_A as f32) * 100.0 / 200.0);
                    terminal.write(b"% done.").unwrap();
                    GLOBAL_FLOAT_A += avg_pitch * 0.005;
                    GLOBAL_INCREMENT_A += 1;
                }
            } else {
                if GLOBAL_INCREMENT_A >= 200 {
                    GLOBAL_INCREMENT_B += 1;
                    GLOBAL_INCREMENT_A = 0;
                    GLOBAL_FLOAT_A = 0.0;
                } else {
                    if !GLOBAL_BOOL_A {
                        print!("Please hold the '{}' button.\n", BUTTONS.get(GLOBAL_INCREMENT_B).unwrap());
                        GLOBAL_BOOL_A = true;
                    }
                }
            }         
        } 
        // Calibrating Screen (below) L: Horizontal || R: Vertical
        else if GLOBAL_INCREMENT_B >= 12 && GLOBAL_INCREMENT_B < 16 {
            if avg_pitch > 50.0 { // if touching screen
                if GLOBAL_INCREMENT_A >= 200 { // if already enough samples
                    if GLOBAL_BOOL_A {
                        let mut terminal = TERM.clone().unwrap();
                        terminal.clear_line().unwrap();
                        PITCHES.push(GLOBAL_FLOAT_A);
                        print!("    Frequencies: {:.2}Hz. To continue, let go of screen.\n", GLOBAL_FLOAT_A);
                        GLOBAL_BOOL_A = false;
                    }
                } else {
                    let mut terminal = TERM.clone().unwrap();
                    terminal.clear_line().unwrap();
                    terminal.write(b"Calibrating, ").unwrap();
                    print!("{:.2}", (GLOBAL_INCREMENT_A as f32) * 100.0 / 200.0);
                    terminal.write(b"% done.").unwrap();
                    GLOBAL_FLOAT_A += if GLOBAL_INCREMENT_B % 2 == 0 {r_pitch} else {l_pitch} * 0.005;
                    GLOBAL_INCREMENT_A += 1;
                }
            } else { 
                if GLOBAL_INCREMENT_A >= 200 {
                    GLOBAL_INCREMENT_B += 1;
                    GLOBAL_INCREMENT_A = 0;
                    GLOBAL_FLOAT_A = 0.0;
                } else {
                    if !GLOBAL_BOOL_A {
                        print!("Please hold the stylus on the '{}' part of the screen.\n", BUTTONS.get(GLOBAL_INCREMENT_B).unwrap());
                        GLOBAL_BOOL_A = true;
                    }
                }
            } 
        } 
        // Calibration finished! Write all of it to a JSON file, do the hover thing, and exit program
        else {
            if avg_pitch < 50.0 { // if not touching screen
                if !GLOBAL_BOOL_B {
                    if !GLOBAL_BOOL_A {
                        let mut json_file = json::JsonValue::new_object();
                        for i in 0..16 {
                            json_file[*BUTTONS.get(i).unwrap()] =(*PITCHES.get(i).unwrap()).into();
                        }
                        let mut file = File::create("pitches.json").unwrap();
                        file.write_all(json_file.pretty(4).as_bytes());
                        print!("Finished pitch calibration!\n");
                    }
                }

                let mouse_style = Style::new().bold().underlined();

                if GLOBAL_INCREMENT_B == 16 {
                    if !GLOBAL_BOOL_A {
                        print!("Hover the cursor on the {} corner of the emulator window and press any button on the controller.\n", mouse_style.apply_to("upper-left"));
                        GLOBAL_BOOL_A = true;
                    }
                } else if GLOBAL_INCREMENT_B == 17 {
                    if !GLOBAL_BOOL_A {
                        print!("Hover the cursor on the {} corner of the emulator window and press any button on the controller.\n", mouse_style.apply_to("lower-right"));
                        GLOBAL_BOOL_A = true;
                    }
                } else {
                    print!("Calibration finished! Closing program.");
                    let mut json_mouse = JsonValue::new_object();
                    json_mouse["left"] = (*MOUSECOORDS.get(0).unwrap()).into();
                    json_mouse["top"] = (*MOUSECOORDS.get(1).unwrap()).into();
                    json_mouse["right"] = (*MOUSECOORDS.get(2).unwrap()).into();
                    json_mouse["bottom"] = (*MOUSECOORDS.get(3).unwrap()).into();
                    let mut mouse_file = File::create("coords.json").unwrap();
                    mouse_file.write_all(json_mouse.pretty(4).as_bytes()).unwrap();
                    CONTINUE_TO_RUN_PROGRAM = false;
                }
            } else {
                if GLOBAL_BOOL_A {
                    let pos = Mouse::get_mouse_position();
                    match pos {
                        Mouse::Position { x, y } => {MOUSECOORDS.push(x); MOUSECOORDS.push(y)},
                        Mouse::Error => panic!("Error during mouse position capture :("),
                    }
                    GLOBAL_INCREMENT_B += 1;
                    GLOBAL_BOOL_A = false;
                }
            }  
        }
    }
}



fn main() {
    let args: Vec<String> = env::args().collect();
    
    unsafe { TERM = Some(Term::stdout()); }

    // Setting flags
    let mut flags = 0;
    if args.contains(&String::from("-c")) { flags |= FlagValues::Calibrating as u128 }
    if args.contains(&String::from("-a")) { flags |= FlagValues::HideInputDevices as u128 }
    if args.contains(&String::from("-hz")) { flags |= FlagValues::SeeButtonFrequency as u128 }
    if args.contains(&String::from("-sp")) { flags |= FlagValues::SkipPitch as u128 }
    if args.contains(&String::from("-x")) { flags |= FlagValues::DisableControl as u128 }

    // Create microphone
    let chosen_mic_id = 0; // <- change this to choose another mic if needed
    let host = cpal::default_host();
    let devices = host.input_devices().unwrap();

    if flags & FlagValues::HideInputDevices as u128 == 0 { print!("Available input devices:\n"); }

    let mut i = 0;
    for device in devices {
        let dev_name = device.name().unwrap();
        let sconfig:StreamConfig = device.default_input_config().unwrap().into();
        let channel_style = if sconfig.channels == 2 {Style::new().green()} else {Style::new()};
        if flags & FlagValues::HideInputDevices as u128 == 0 {
            print!("#{}: ({} ch(s)) \"{}\"\n\n", channel_style.apply_to(i), channel_style.apply_to(sconfig.channels), channel_style.apply_to(dev_name));
        }
        i += 1;
    }

    let microphone = host.input_devices().unwrap().nth(chosen_mic_id).unwrap();
    let microphone_name =  microphone.name().unwrap();

    print!("Chosen device: \"{}\"\n", microphone_name);

    // Build Config

    let sconfig:StreamConfig = microphone.default_input_config().unwrap().into();
    if sconfig.channels != 2 {
        panic!("RPBC requires stereo input to function.\n");
    }
    

    // Build detector
    let mut detector = HannedFftDetector::default();

    // If not calibrating, fill global vector with pitches and coords
    if flags & FlagValues::Calibrating as u128 == 0 {
        let json_file_schrodinger = File::open("pitches.json");
        let mut json_file = match json_file_schrodinger {
            Ok(json_file_cat) => json_file_cat,
            Err(err) => panic!("Warning: no pitches.json file found. Run program with -c flag to calibrate and generate file.")
        };
        let mut contents = String::new();
        json_file.read_to_string(&mut contents).unwrap();
        let json_obj = json::parse(&contents).unwrap();
        let json_entries = json_obj.entries();
        for (button, pitch) in json_entries {
            unsafe {
                PITCHES.push(pitch.as_f64().unwrap());
            }
        }

        let json_file_schrodinger = File::open("coords.json");
        json_file = match json_file_schrodinger {
            Ok(json_file_cat) => json_file_cat,
            Err(err) => panic!("Warning: no coords.json file found. Run program with -c flag to calibrate and generate file. (TIP: Also use -sp flag to skip pitch-setting :) )")
        };
        contents = String::new();
        json_file.read_to_string(&mut contents).unwrap();
        let json_obj = json::parse(&contents).unwrap();
        let json_entries = json_obj.entries();
        for (spot, coord) in json_entries {
            unsafe {
                MOUSECOORDS.push(coord.as_i32().unwrap());
            }
        }
        
    }

    // Set up stream
    let mut r_channel: Vec<f64> = Vec::new();
    let mut l_channel: Vec<f64> = Vec::new();
    let mut r_pitch:f64 = 0.0;
    let mut l_pitch:f64 = 0.0;

    let stream_schrodinger = microphone.build_input_stream(
        &sconfig, 
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            r_channel.clear();
            l_channel.clear();
            let mut l_sn:f64 = 0.0;
            let mut r_sn:f64 = 0.0;
            for i in 0..255 {
                l_sn = f64::from(data[4*i]);
                r_sn = f64::from(data[4*i+1]);
                l_channel.push(l_sn);
                r_channel.push(r_sn);
                l_sn = f64::from(data[4*i+2]);
                r_sn = f64::from(data[4*i+3]);
                l_channel.push(l_sn);
                r_channel.push(r_sn);
            }
            r_pitch = detector.detect_pitch(&r_channel, 44100.0).unwrap();
            l_pitch = detector.detect_pitch(&l_channel, 44100.0).unwrap();
            
            if flags & FlagValues::Calibrating as u128 == 0 {
                pitch_functionality(&l_pitch, &r_pitch, &flags);
            } else {
                calibration_pitch_functionality(&l_pitch, &r_pitch, &flags);
            }
        },
        move |err| {
            print!("not success :( -> {}", err);
        },
        None
    );

    let stream = match stream_schrodinger {
        Ok(stream_cat) => stream_cat,
        Err(err) => panic!("idk girl -> {}", err),
    };

    stream.play().unwrap();

    unsafe { while CONTINUE_TO_RUN_PROGRAM {

    } }

    
    // print smth idk
}

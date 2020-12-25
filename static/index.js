import init, { Cpu } from './pkg/chip8_emu.js';

let CLOCK_RATE_HZ = 600;
let USE_ORIGINAL_SHIFT = false;
let USE_ORIGINAL_MEM_ACC = false;

// === Screen output ===
const canvas = document.getElementById("game_screen");
const ctx = canvas.getContext('2d');
// Clear screen
ctx.fillStyle = "black";
ctx.fillRect(0, 0, canvas.width, canvas.height);
let last_animation_request_id;

// === Keyboard input state ===
const key_state = [];
for (let i = 0; i < 16; i++) {
    key_state.push(false);
}
// True if we are waiting for a keypress
let should_capture_key = false;
// True if a keypress was capture and is waiting to be sent to the cpu
let finished_capture_key = false;
let captured_key;

// === Audio output state ===
const audio_context = new (window.AudioContext || window.webkitAudioContext)();
const master_gain = audio_context.createGain();
master_gain.connect(audio_context.destination);
const oscillator = new OscillatorNode(audio_context, { type: 'triangle' });
let tone_playing = false;
let audio_initiated = false;

// Built-in ROMs state
const rom_descriptions = [];

// CHIP-8 State
let wasm;
let chip8_cpu;
let loaded_rom_buffer;

init_wasm();

async function init_wasm() {
    wasm = await init();
    setup_event_listeners();
    populate_builtin_roms();
}

async function populate_builtin_roms() {
    const rom_list = await fetch('./roms/roms.json').then(resp => resp.json());
    const rom_select = document.getElementById("rom_select");

    for(let rom_data of rom_list){
        const rom_option = document.createElement('option');
        rom_option.value = rom_option.text = rom_data.file;
        rom_select.add(rom_option);

        rom_descriptions.push(rom_data.desc);
    }

    rom_select.removeChild(document.getElementById("downloading_option"));
}

let last_frame_timestamp;
function render_loop(timestamp) {
    if (last_frame_timestamp == undefined) {
        last_frame_timestamp = timestamp;
        last_animation_request_id = requestAnimationFrame(render_loop);
        return;
    }

    const delta_time = timestamp - last_frame_timestamp;
    // TODO: Handle non 60FPS animation
    chip8_cpu.tick_clock();

    // console.log(key_state);
    chip8_cpu.update_key_state(key_state);
    if (finished_capture_key) {
        finished_capture_key = false;
        chip8_cpu.set_captured_key(captured_key);
    }

    let cpu_start = performance.now();

    // Check we are not waiting for a key press capture
    if (!should_capture_key) {
        let cpu_start = performance.now();
        const instructions_to_execute = Math.max(Math.round((delta_time / 1000) * CLOCK_RATE_HZ), 1);
        for (let i = 0; i < instructions_to_execute; i++) {
            chip8_cpu.step();

            if (chip8_cpu.is_waiting_for_keypress()) {
                should_capture_key = true;
                break;
            }

            // Check if the executed instruction changed the screen
            if (chip8_cpu.handle_screen_dirty_flag()) {
                const screen_buffer_ptr = chip8_cpu.get_screen_buffer();
                const screen_buffer = new Uint8Array(wasm.memory.buffer, screen_buffer_ptr, 64 * 32);

                const image_data = ctx.getImageData(0, 0, canvas.width, canvas.height);
                const data = image_data.data;

                let buffer_idx = 0;
                for (let i = 0; i < data.length; i += 4) {
                    let pixelColor = (screen_buffer[buffer_idx] > 0) ? 255 : 0;
                    data[i] = pixelColor; // red
                    data[i + 1] = pixelColor; // green
                    data[i + 2] = pixelColor; // blue

                    buffer_idx++;
                }
                ctx.putImageData(image_data, 0, 0);

                // According to the reference, draw instruction waited for a v-blank.
                break;
            }
        }
    }

    let cpu_end = performance.now();
    // console.log(`Took ${cpu_end - cpu_start} ms to execute instructions.`);

    let should_play_tone = chip8_cpu.should_play_tone();
    if (should_play_tone && !tone_playing) {
        oscillator.connect(master_gain);
        tone_playing = true;
    } else if (!should_play_tone && tone_playing) {
        oscillator.disconnect(master_gain);
        tone_playing = false;
    }

    last_frame_timestamp = timestamp;
    last_animation_request_id = requestAnimationFrame(render_loop);
};

function start_game() {
    stop_game();

    if(!audio_initiated) {
        oscillator.start();
        audio_initiated = true;
    }

    // Setup cpu
    if (loaded_rom_buffer.byteLength == 0) {
        console.error("Failed to load ROM");
        ctx.font = "11px Courier New";
        ctx.fillStyle = "white";
        ctx.fillText("Failed to", 1, 8);
        ctx.fillText("load ROM!", 1, 17);
        return;
    }
    chip8_cpu = Cpu.with_rom_and_options(new Uint8Array(loaded_rom_buffer),
        USE_ORIGINAL_SHIFT, USE_ORIGINAL_MEM_ACC);

    last_animation_request_id = requestAnimationFrame(render_loop);
}

function stop_game() {
    // Stop animation loop
    if(last_animation_request_id != undefined){
        cancelAnimationFrame(last_animation_request_id);
    }

    // Clear screen
    ctx.fillStyle = "black";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Reset key state
    should_capture_key = false;
    finished_capture_key = false;

    // Reset frame timing state
    last_frame_timestamp = undefined;
}

function set_loaded_rom_buffer(rom_name, rom_buffer){
    document.getElementById("rom_filename").innerText = rom_name;
    loaded_rom_buffer = rom_buffer;
    document.getElementById("start_game").disabled = false;
}

function show_loading_rom() {
    document.getElementById("rom_filename").innerText = "Loading...";
}

function handle_rom_upload(files){
    if(!files || files.length == 0)
        return;
    
    let rom_file = files[0];
    if(rom_file.size < 4096-512) { // Memory size - reserved memory
        const file_reader = new FileReader();
        file_reader.onload = () => {
            stop_game();
            show_loading_rom();
            set_loaded_rom_buffer(rom_file.name, file_reader.result);
        };
        file_reader.onerror = (err) => {
            alert(`Failed to read ROM file. Error: ${err.message}`);
        } 
        file_reader.readAsArrayBuffer(rom_file);
    }else{
        alert("ROM file too large!");
    }
}

function setup_event_listeners() {
    const key_map = {
        '1': 0x1, '2': 0x2, '3': 0x3, '4': 0xC,
        'Q': 0x4, 'W': 0x5, 'E': 0x6, 'R': 0xD,
        'A': 0x7, 'S': 0x8, 'D': 0x9, 'F': 0xE,
        'Z': 0xA, 'X': 0x0, 'C': 0xB, 'V': 0xF,
    };

    const handle_keydown = (key_digit) => {
        key_state[key_digit] = true;
        document.getElementById("key_"+key_digit.toString(16).toUpperCase()).classList.add("key_button_pressed");
    };
    const handle_keyup = (key_digit) => {
        key_state[key_digit] = false;
            document.getElementById("key_"+key_digit.toString(16).toUpperCase()).classList.remove("key_button_pressed");

            if (should_capture_key) {
                should_capture_key = false;
                finished_capture_key = true;
                captured_key = key_digit;
            }
    };

    document.addEventListener("keydown", ev => {
        let key = get_key_from_event(ev);

        if (key in key_map) {
            let key_digit = key_map[key];
            handle_keydown(key_digit);
        }
    });

    document.addEventListener("keyup", ev => {
        let key = get_key_from_event(ev);

        if (key in key_map) {
            let key_digit = key_map[key];
            handle_keyup(key_digit)
        }
    });

    for(let i = 0; i < 16; i++){
        document.getElementById("key_"+i.toString(16).toUpperCase()).addEventListener("mousedown", ev => {
            handle_keydown(i);
        });
        document.getElementById("key_"+i.toString(16).toUpperCase()).addEventListener("mouseup", ev => {
            handle_keyup(i);
        });
        document.getElementById("key_"+i.toString(16).toUpperCase()).addEventListener("mouseleave", ev => {
            handle_keyup(i);
        });
    }

    document.getElementById("start_game").addEventListener("click", async () => {
        start_game();
    });

    const mute_button = document.getElementById("mute");
    mute_button.addEventListener("click", () => {
        master_gain.gain.value = 1 - master_gain.gain.value;
        if (master_gain.gain.value == 1) {
            mute_button.innerText = "Mute Sound";
        } else {
            mute_button.innerText = "Unmute Sound";
        }
    });

    document.getElementById("load_select_rom").addEventListener("click", async () => {
        const rom_select = document.getElementById("rom_select");
        if(rom_select.value.endsWith(".rom")){
            show_loading_rom();
            set_loaded_rom_buffer(rom_select.value, await fetch('./roms/'+rom_select.value).then(resp => resp.arrayBuffer()));

            document.getElementById("rom_description").innerText = rom_descriptions[rom_select.selectedIndex];
        }
    });

    const file_picker = document.getElementById("file_picker");
    file_picker.addEventListener("change", async ev => {
        let files = ev.target.files;
        handle_rom_upload(files);
    });
    const upload_rom_button = document.getElementById("upload_rom");
    upload_rom_button.addEventListener("click", () => {
        file_picker.click();
    });
    upload_rom_button.addEventListener("dragenter", ev => {
        ev.stopPropagation();
        ev.preventDefault();
        
        ev.target.classList.add("upload_dragenter");
    });
    upload_rom_button.addEventListener("dragover", ev => {
        ev.stopPropagation();
        ev.preventDefault();
    });
    upload_rom_button.addEventListener("dragleave", ev => {
        ev.stopPropagation();
        ev.preventDefault();
        
        ev.target.classList.remove("upload_dragenter");
    });
    upload_rom_button.addEventListener("drop", ev => {
        ev.stopPropagation();
        ev.preventDefault();
        
        ev.target.classList.remove("upload_dragenter");
        handle_rom_upload(ev.dataTransfer.files);
    });

    const keyboard_label = document.getElementById("keyboard_label");
    keyboard_label.addEventListener("mouseenter", () => {
        for(let i = 0; i < 16; i++){
            let key_name = i.toString(16).toUpperCase();
            document.getElementById("key_"+key_name).innerText =
                Object.keys(key_map).find(key => key_map[key] == i);
        }
    });
    keyboard_label.addEventListener("mouseleave", () => {
        for(let i = 0; i < 16; i++){
            let key_name = i.toString(16).toUpperCase();
            document.getElementById("key_"+key_name).innerText = key_name;
        }
    });

    document.getElementById("clock_rate").addEventListener("input", ev => {
        CLOCK_RATE_HZ = parseInt(ev.target.value);
    });

    document.getElementById("advanced_settings_title").addEventListener("click", () => {
        const panel = document.getElementById("advanced_settings_panel");
        const advanced_fold = document.getElementById("advanced_fold");
        if(panel.style.display == 'none'){
            panel.style.display = 'block';
            advanced_fold.innerText = '▲';
        }else{
            panel.style.display = 'none';
            advanced_fold.innerText = '▼';
        }
    });

    document.getElementById("original_shift").addEventListener("change", ev => {
        stop_game();
        USE_ORIGINAL_SHIFT = ev.target.checked;
    });
    document.getElementById("original_mem_acc").addEventListener("change", ev => {
        stop_game();
        USE_ORIGINAL_MEM_ACC = ev.target.checked;
    });
}

function get_key_from_event(ev) {
    let key;
    if(ev.code.startsWith("Digit")){
        key = ev.code.substr(5);
    }else if(ev.code.startsWith("Key")){
        key = ev.code.substr(3);
    }else{
        key = ev.key;
    }
    return key.toUpperCase();
}
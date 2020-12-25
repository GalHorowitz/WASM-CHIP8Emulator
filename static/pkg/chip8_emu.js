
let wasm;

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachegetUint8Memory0 = null;
function getUint8Memory0() {
    if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1);
    getUint8Memory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

let cachedTextEncoder = new TextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
}

function handleError(f) {
    return function () {
        try {
            return f.apply(this, arguments);

        } catch (e) {
            wasm.__wbindgen_exn_store(addHeapObject(e));
        }
    };
}

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}
/**
* Represents a CHIP-8 CPU
*/
export class Cpu {

    static __wrap(ptr) {
        const obj = Object.create(Cpu.prototype);
        obj.ptr = ptr;

        return obj;
    }

    free() {
        const ptr = this.ptr;
        this.ptr = 0;

        wasm.__wbg_cpu_free(ptr);
    }
    /**
    * Construct a CHIP-8 cpu at the intial entry state.
    * @returns {Cpu}
    */
    static new() {
        var ret = wasm.cpu_new();
        return Cpu.__wrap(ret);
    }
    /**
    * Construct a CHIP-8 cpu at the initial entry state, with rom bytes loaded at the entry point
    * in memory.
    * @param {Uint8Array} rom
    * @returns {Cpu}
    */
    static with_rom(rom) {
        var ptr0 = passArray8ToWasm0(rom, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        var ret = wasm.cpu_with_rom(ptr0, len0);
        return Cpu.__wrap(ret);
    }
    /**
    * Construct a CHIP-8 cou at the initial entry state, with rom bytes loaded at the entry point
    * in memory.
    * When `original_shift` is true, the original behaviour of the shift instructions is used,
    * i.e. the instruction shifts Vy instead of Vx.
    * When `original_mem_acc` is true, the original behaviour of the load/store instructions is
    * used, i.e. the instructions increment the I register by the number of registers used.
    * @param {Uint8Array} rom
    * @param {boolean} original_shift
    * @param {boolean} original_mem_acc
    * @returns {Cpu}
    */
    static with_rom_and_options(rom, original_shift, original_mem_acc) {
        var ptr0 = passArray8ToWasm0(rom, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        var ret = wasm.cpu_with_rom_and_options(ptr0, len0, original_shift, original_mem_acc);
        return Cpu.__wrap(ret);
    }
    /**
    * Decode and execute one instruction.
    * It is the responsibility of the caller to check the `waiting_for_keypress` flag. If it is
    * set, The caller should only call `step` again after calling `set_captured_key`.
    * It is the responsibility of the caller to check the `screen_dirty` flag and update the
    * display if needed.
    */
    step() {
        wasm.cpu_step(this.ptr);
    }
    /**
    * Tick internal cpu timers. Must be called at 60HZ.
    */
    tick_clock() {
        wasm.cpu_tick_clock(this.ptr);
    }
    /**
    * Get a pointer to the screen buffer memory, used from the JS side to render the screen.
    * @returns {number}
    */
    get_screen_buffer() {
        var ret = wasm.cpu_get_screen_buffer(this.ptr);
        return ret;
    }
    /**
    * Returns whether or not the screen dirty, and if it is, sets it to false.
    * @returns {boolean}
    */
    handle_screen_dirty_flag() {
        var ret = wasm.cpu_handle_screen_dirty_flag(this.ptr);
        return ret !== 0;
    }
    /**
    * Update the internal key state to the provided key state.
    * `new_key_state` must be of length 16.
    * @param {Uint8Array} new_key_state
    */
    update_key_state(new_key_state) {
        var ptr0 = passArray8ToWasm0(new_key_state, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.cpu_update_key_state(this.ptr, ptr0, len0);
    }
    /**
    * Returns true if the cpu is waiting for a captured key press.
    * @returns {boolean}
    */
    is_waiting_for_keypress() {
        var ret = wasm.cpu_is_waiting_for_keypress(this.ptr);
        return ret !== 0;
    }
    /**
    * Sets the key that was captured in the last keypress.
    * @param {number} captured_key
    */
    set_captured_key(captured_key) {
        wasm.cpu_set_captured_key(this.ptr, captured_key);
    }
    /**
    * Returns true if the emulator should play a tone
    * @returns {boolean}
    */
    should_play_tone() {
        var ret = wasm.cpu_should_play_tone(this.ptr);
        return ret !== 0;
    }
}

async function load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {

        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

async function init(input) {
    if (typeof input === 'undefined') {
        input = import.meta.url.replace(/\.js$/, '_bg.wasm');
    }
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_new_59cb74e423758ede = function() {
        var ret = new Error();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_stack_558ba5917b466edd = function(arg0, arg1) {
        var ret = getObject(arg1).stack;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_error_4bb6c2a97407129a = function(arg0, arg1) {
        try {
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(arg0, arg1);
        }
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_self_1c83eb4471d9eb9b = handleError(function() {
        var ret = self.self;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_require_5b2b5b594d809d9f = function(arg0, arg1, arg2) {
        var ret = getObject(arg0).require(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_crypto_c12f14e810edcaa2 = function(arg0) {
        var ret = getObject(arg0).crypto;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_msCrypto_679be765111ba775 = function(arg0) {
        var ret = getObject(arg0).msCrypto;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        var ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbg_getRandomValues_05a60bf171bfc2be = function(arg0) {
        var ret = getObject(arg0).getRandomValues;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getRandomValues_3ac1b33c90b52596 = function(arg0, arg1, arg2) {
        getObject(arg0).getRandomValues(getArrayU8FromWasm0(arg1, arg2));
    };
    imports.wbg.__wbg_randomFillSync_6f956029658662ec = function(arg0, arg1, arg2) {
        getObject(arg0).randomFillSync(getArrayU8FromWasm0(arg1, arg2));
    };
    imports.wbg.__wbg_static_accessor_MODULE_abf5ae284bffdf45 = function() {
        var ret = module;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }

    const { instance, module } = await load(await input, imports);

    wasm = instance.exports;
    init.__wbindgen_wasm_module = module;

    return wasm;
}

export default init;


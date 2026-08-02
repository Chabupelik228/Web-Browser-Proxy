// frontend/public/config.js

const xorCodec = {
    encode: (str) => {
        if (!str) return str;
        return encodeURIComponent(
            str.toString().split('').map((char, ind) =>
                ind % 2 ? String.fromCharCode(char.charCodeAt(0) ^ 2) : char
            ).join('')
        );
    },
    decode: (str) => {
        if (!str) return str;
        let [input, ...search] = str.split('?');
        return decodeURIComponent(input).split('').map((char, ind) =>
            ind % 2 ? String.fromCharCode(char.charCodeAt(0) ^ 2) : char
        ).join('') + (search.length ? '?' + search.join('?') : '');
    }
};

const scramjetConfig = {
    prefix: '/service/',
    bare: '/bare/',

    files: {
        wasm: '/scram/scramjet.wasm.wasm',
        all: '/scram/scramjet.all.js',
        sync: '/scram/scramjet.sync.js',
    },

    codec: xorCodec,

    wsConfig: {
        wisp: (location.protocol === 'https:' ? 'wss://' : 'ws://') + location.host + '/wisp/'
    }
};

self.__scramjet$config = scramjetConfig;
self.__uv$config = scramjetConfig;
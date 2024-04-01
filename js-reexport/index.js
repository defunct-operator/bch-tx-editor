import { cashAssemblyToBin, disassembleBytecodeBCH } from '@bitauth/libauth';

let reexports = {};

reexports.cashAssemblyToBin = function(script) {
    let result = cashAssemblyToBin(script);
    if (result instanceof Uint8Array) {
        return result;
    } else {
        throw result;
    }
}

reexports.disassembleBytecodeBCH = disassembleBytecodeBCH;

window.reexports = reexports;

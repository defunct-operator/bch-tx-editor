// @ts-check
import { cashAssemblyToBin, createCompilerBCH, disassembleBytecodeBCH } from '@bitauth/libauth';

let reexports = {};

const compiler = createCompilerBCH({ scripts: {}, operations: {} });
/**
 * @param {string} script
 * @returns Uint8Array
 */
reexports.cashAssemblyToBin = function(script) {
    compiler.configuration.scripts["script"] = script;
    const result = compiler.generateBytecode({ data: {}, scriptId: "script" });
    if (result.success) {
        return result.bytecode;
    } else {
        throw `CashAssembly compilation ${result.errorType} error: ${result.errors
            .map((err) => err.error)
            .join(' ')}`;
    }
};

reexports.disassembleBytecodeBCH = disassembleBytecodeBCH;

// @ts-ignore
window.reexports = reexports;

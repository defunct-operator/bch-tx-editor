// @ts-check
import { createCompilerBch, decodeAuthenticationInstructions, disassembleAuthenticationInstructionsMaybeMalformed, OpcodesBch } from '@bitauth/libauth';

let reexports = {};

const compiler = createCompilerBch({ scripts: {}, operations: {} });
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

/**
 * @param {Uint8Array} bytecode
 * @returns string
 */
reexports.disassembleBytecodeBCH = function(bytecode) {
    return disassembleAuthenticationInstructionsMaybeMalformed(OpcodesBch, decodeAuthenticationInstructions(bytecode));
};

// @ts-ignore
window.reexports = reexports;

let U=0,W=`string`,Q=1,Y=`Object`,S=`utf-8`,P=null,R=`undefined`,$=4,X=`function`,_=16,N=Array,T=Error,Z=FinalizationRegistry,a1=Promise,a0=Reflect,V=Uint8Array,O=undefined;var H=(async(a,b)=>{if(typeof Response===X&&a instanceof Response){if(typeof WebAssembly.instantiateStreaming===X){try{return await WebAssembly.instantiateStreaming(a,b)}catch(b){if(a.headers.get(`Content-Type`)!=`application/wasm`){console.warn(`\`WebAssembly.instantiateStreaming\` failed because your server does not serve wasm with \`application/wasm\` MIME type. Falling back to \`WebAssembly.instantiate\` which is slower. Original error:\\n`,b)}else{throw b}}};const c=await a.arrayBuffer();return await WebAssembly.instantiate(c,b)}else{const c=await WebAssembly.instantiate(a,b);if(c instanceof WebAssembly.Instance){return {instance:c,module:a}}else{return c}}});var l=(a=>{const b=typeof a;if(b==`number`||b==`boolean`||a==P){return `${a}`};if(b==W){return `"${a}"`};if(b==`symbol`){const b=a.description;if(b==P){return `Symbol`}else{return `Symbol(${b})`}};if(b==X){const b=a.name;if(typeof b==W&&b.length>U){return `Function(${b})`}else{return `Function`}};if(N.isArray(a)){const b=a.length;let c=`[`;if(b>U){c+=l(a[U])};for(let d=Q;d<b;d++){c+=`, `+ l(a[d])};c+=`]`;return c};const c=/\[object ([^\]]+)\]/.exec(toString.call(a));let d;if(c.length>Q){d=c[Q]}else{return toString.call(a)};if(d==Y){try{return `Object(`+ JSON.stringify(a)+ `)`}catch(a){return Y}};if(a instanceof T){return `${a.name}: ${a.message}\n${a.stack}`};return d});var J=((a,b)=>{});var u=((b,c,d)=>{a.__wbindgen_export_3(b,c,e(d))});var g=(a=>{const b=c(a);f(a);return b});var e=(a=>{if(d===b.length)b.push(b.length+ Q);const c=d;d=b[c];b[c]=a;return c});var A=((b,c,d,f)=>{a.__wbindgen_export_8(b,c,e(d),e(f))});function z(b,c){try{return b.apply(this,c)}catch(b){a.__wbindgen_export_7(e(b))}}var x=((a,b)=>{if(a===U){return c(b)}else{return k(a,b)}});var r=(()=>{if(q===P||q.byteLength===U){q=new Int32Array(a.memory.buffer)};return q});var y=(a=>a===O||a===P);var c=(a=>b[a]);var M=(async(b)=>{if(a!==O)return a;if(typeof b===R){b=new URL(`bch-tx-editor-85db17add49e2e51_bg.wasm`,import.meta.url)};const c=I();if(typeof b===W||typeof Request===X&&b instanceof Request||typeof URL===X&&b instanceof URL){b=fetch(b)};J(c);const {instance:d,module:e}=await H(await b,c);return K(d,e)});var v=((b,c,d)=>{a.__wbindgen_export_4(b,c,e(d))});var L=(b=>{if(a!==O)return a;const c=I();J(c);if(!(b instanceof WebAssembly.Module)){b=new WebAssembly.Module(b)};const d=new WebAssembly.Instance(b,c);return K(d,b)});var K=((b,c)=>{a=b.exports;M.__wbindgen_wasm_module=c;q=P;i=P;a.__wbindgen_start();return a});var I=(()=>{const b={};b.wbg={};b.wbg.__wbindgen_object_clone_ref=(a=>{const b=c(a);return e(b)});b.wbg.__wbindgen_object_drop_ref=(a=>{g(a)});b.wbg.__wbindgen_string_new=((a,b)=>{const c=k(a,b);return e(c)});b.wbg.__wbindgen_number_new=(a=>{const b=a;return e(b)});b.wbg.__wbindgen_bigint_from_u64=(a=>{const b=BigInt.asUintN(64,a);return e(b)});b.wbg.__wbg_new_abda76e883ba8a5f=(()=>{const a=new T();return e(a)});b.wbg.__wbg_stack_658279fe44541cf6=((b,d)=>{const e=c(d).stack;const f=p(e,a.__wbindgen_export_0,a.__wbindgen_export_1);const g=m;r()[b/$+ Q]=g;r()[b/$+ U]=f});b.wbg.__wbg_error_f851667af71bcfc6=((b,c)=>{var d=x(b,c);if(b!==U){a.__wbindgen_export_6(b,c,Q)};console.error(d)});b.wbg.__wbindgen_is_undefined=(a=>{const b=c(a)===O;return b});b.wbg.__wbindgen_is_null=(a=>{const b=c(a)===P;return b});b.wbg.__wbindgen_is_falsy=(a=>{const b=!c(a);return b});b.wbg.__wbindgen_cb_drop=(a=>{const b=g(a).original;if(b.cnt--==Q){b.a=U;return !0};const c=!1;return c});b.wbg.__wbg_instanceof_Window_f401953a2cf86220=(a=>{let b;try{b=c(a) instanceof Window}catch(a){b=!1}const d=b;return d});b.wbg.__wbg_document_5100775d18896c16=(a=>{const b=c(a).document;return y(b)?U:e(b)});b.wbg.__wbg_body_edb1908d3ceff3a1=(a=>{const b=c(a).body;return y(b)?U:e(b)});b.wbg.__wbg_createComment_354ccab4fdc521ee=((a,b,d)=>{var f=x(b,d);const g=c(a).createComment(f);return e(g)});b.wbg.__wbg_createDocumentFragment_8c86903bbb0a3c3c=(a=>{const b=c(a).createDocumentFragment();return e(b)});b.wbg.__wbg_createElement_8bae7856a4bb7411=function(){return z(((a,b,d)=>{var f=x(b,d);const g=c(a).createElement(f);return e(g)}),arguments)};b.wbg.__wbg_createTextNode_0c38fd80a5b2284d=((a,b,d)=>{var f=x(b,d);const g=c(a).createTextNode(f);return e(g)});b.wbg.__wbg_classList_1f0528ee002e56d4=(a=>{const b=c(a).classList;return e(b)});b.wbg.__wbg_setinnerHTML_26d69b59e1af99c7=((a,b,d)=>{var e=x(b,d);c(a).innerHTML=e});b.wbg.__wbg_removeAttribute_1b10a06ae98ebbd1=function(){return z(((a,b,d)=>{var e=x(b,d);c(a).removeAttribute(e)}),arguments)};b.wbg.__wbg_setAttribute_3c9f6c303b696daa=function(){return z(((a,b,d,e,f)=>{var g=x(b,d);var h=x(e,f);c(a).setAttribute(g,h)}),arguments)};b.wbg.__wbg_before_210596e44d88649f=function(){return z(((a,b)=>{c(a).before(c(b))}),arguments)};b.wbg.__wbg_remove_49b0a5925a04b955=(a=>{c(a).remove()});b.wbg.__wbg_append_fcf463f0b4a8f219=function(){return z(((a,b)=>{c(a).append(c(b))}),arguments)};b.wbg.__wbg_add_dcb05a8ba423bdac=function(){return z(((a,b,d)=>{var e=x(b,d);c(a).add(e)}),arguments)};b.wbg.__wbg_remove_698118fb25ab8150=function(){return z(((a,b,d)=>{var e=x(b,d);c(a).remove(e)}),arguments)};b.wbg.__wbg_addEventListener_53b787075bd5e003=function(){return z(((a,b,d,e)=>{var f=x(b,d);c(a).addEventListener(f,c(e))}),arguments)};b.wbg.__wbg_addEventListener_4283b15b4f039eb5=function(){return z(((a,b,d,e,f)=>{var g=x(b,d);c(a).addEventListener(g,c(e),c(f))}),arguments)};b.wbg.__wbg_checked_749a34774f2df2e3=(a=>{const b=c(a).checked;return b});b.wbg.__wbg_value_47fe6384562f52ab=((b,d)=>{const e=c(d).value;const f=p(e,a.__wbindgen_export_0,a.__wbindgen_export_1);const g=m;r()[b/$+ Q]=g;r()[b/$+ U]=f});b.wbg.__wbg_byobRequest_72fca99f9c32c193=(a=>{const b=c(a).byobRequest;return y(b)?U:e(b)});b.wbg.__wbg_close_184931724d961ccc=function(){return z((a=>{c(a).close()}),arguments)};b.wbg.__wbg_close_a994f9425dab445c=function(){return z((a=>{c(a).close()}),arguments)};b.wbg.__wbg_enqueue_ea194723156c0cc2=function(){return z(((a,b)=>{c(a).enqueue(c(b))}),arguments)};b.wbg.__wbg_setdata_8c2b43af041cc1b3=((a,b,d)=>{var e=x(b,d);c(a).data=e});b.wbg.__wbg_parentNode_6be3abff20e1a5fb=(a=>{const b=c(a).parentNode;return y(b)?U:e(b)});b.wbg.__wbg_childNodes_118168e8b23bcb9b=(a=>{const b=c(a).childNodes;return e(b)});b.wbg.__wbg_previousSibling_9708a091a3e6e03b=(a=>{const b=c(a).previousSibling;return y(b)?U:e(b)});b.wbg.__wbg_nextSibling_709614fdb0fb7a66=(a=>{const b=c(a).nextSibling;return y(b)?U:e(b)});b.wbg.__wbg_settextContent_d271bab459cbb1ba=((a,b,d)=>{var e=x(b,d);c(a).textContent=e});b.wbg.__wbg_appendChild_580ccb11a660db68=function(){return z(((a,b)=>{const d=c(a).appendChild(c(b));return e(d)}),arguments)};b.wbg.__wbg_cloneNode_e19c313ea20d5d1d=function(){return z((a=>{const b=c(a).cloneNode();return e(b)}),arguments)};b.wbg.__wbg_new_c7aa03c061e95bde=function(){return z((()=>{const a=new Range();return e(a)}),arguments)};b.wbg.__wbg_deleteContents_1b5a33e17bc6400f=function(){return z((a=>{c(a).deleteContents()}),arguments)};b.wbg.__wbg_setEndBefore_6d219390ff50f205=function(){return z(((a,b)=>{c(a).setEndBefore(c(b))}),arguments)};b.wbg.__wbg_setStartBefore_2dac025de1f18aa0=function(){return z(((a,b)=>{c(a).setStartBefore(c(b))}),arguments)};b.wbg.__wbg_instanceof_ShadowRoot_9db040264422e84a=(a=>{let b;try{b=c(a) instanceof ShadowRoot}catch(a){b=!1}const d=b;return d});b.wbg.__wbg_host_c667c7623404d6bf=(a=>{const b=c(a).host;return e(b)});b.wbg.__wbg_length_d0a802565d17eec4=(a=>{const b=c(a).length;return b});b.wbg.__wbg_target_2fc177e386c8b7b0=(a=>{const b=c(a).target;return y(b)?U:e(b)});b.wbg.__wbg_cancelBubble_c0aa3172524eb03c=(a=>{const b=c(a).cancelBubble;return b});b.wbg.__wbg_composedPath_58473fd5ae55f2cd=(a=>{const b=c(a).composedPath();return e(b)});b.wbg.__wbg_append_7ba9d5c2eb183eea=function(){return z(((a,b)=>{c(a).append(c(b))}),arguments)};b.wbg.__wbg_view_7f0ce470793a340f=(a=>{const b=c(a).view;return y(b)?U:e(b)});b.wbg.__wbg_respond_b1a43b2e3a06d525=function(){return z(((a,b)=>{c(a).respond(b>>>U)}),arguments)};b.wbg.__wbg_error_8e3928cfb8a43e2b=(a=>{console.error(c(a))});b.wbg.__wbg_log_5bb5f88f245d7762=(a=>{console.log(c(a))});b.wbg.__wbg_warn_63bbae1730aead09=(a=>{console.warn(c(a))});b.wbg.__wbg_queueMicrotask_481971b0d87f3dd4=(a=>{queueMicrotask(c(a))});b.wbg.__wbg_queueMicrotask_3cbae2ec6b6cd3d6=(a=>{const b=c(a).queueMicrotask;return e(b)});b.wbg.__wbindgen_is_function=(a=>{const b=typeof c(a)===X;return b});b.wbg.__wbg_get_bd8e338fbd5f5cc8=((a,b)=>{const d=c(a)[b>>>U];return e(d)});b.wbg.__wbg_newnoargs_e258087cd0daa0ea=((a,b)=>{var c=x(a,b);const d=new Function(c);return e(d)});b.wbg.__wbg_get_e3c254076557e348=function(){return z(((a,b)=>{const d=a0.get(c(a),c(b));return e(d)}),arguments)};b.wbg.__wbg_call_27c0f87801dedf93=function(){return z(((a,b)=>{const d=c(a).call(c(b));return e(d)}),arguments)};b.wbg.__wbg_self_ce0dbfc45cf2f5be=function(){return z((()=>{const a=self.self;return e(a)}),arguments)};b.wbg.__wbg_window_c6fb939a7f436783=function(){return z((()=>{const a=window.window;return e(a)}),arguments)};b.wbg.__wbg_globalThis_d1e6af4856ba331b=function(){return z((()=>{const a=globalThis.globalThis;return e(a)}),arguments)};b.wbg.__wbg_global_207b558942527489=function(){return z((()=>{const a=global.global;return e(a)}),arguments)};b.wbg.__wbg_new_28c511d9baebfa89=((a,b)=>{var c=x(a,b);const d=new T(c);return e(d)});b.wbg.__wbg_call_b3ca7c6051f9bec1=function(){return z(((a,b,d)=>{const f=c(a).call(c(b),c(d));return e(f)}),arguments)};b.wbg.__wbg_is_010fdc0f4ab96916=((a,b)=>{const d=Object.is(c(a),c(b));return d});b.wbg.__wbg_new_81740750da40724f=((a,b)=>{try{var c={a:a,b:b};var d=(a,b)=>{const d=c.a;c.a=U;try{return A(d,c.b,a,b)}finally{c.a=d}};const f=new a1(d);return e(f)}finally{c.a=c.b=U}});b.wbg.__wbg_resolve_b0083a7967828ec8=(a=>{const b=a1.resolve(c(a));return e(b)});b.wbg.__wbg_then_0c86a60e8fcfe9f6=((a,b)=>{const d=c(a).then(c(b));return e(d)});b.wbg.__wbg_buffer_12d079cc21e14bdb=(a=>{const b=c(a).buffer;return e(b)});b.wbg.__wbg_newwithbyteoffsetandlength_aa4a17c33a06e5cb=((a,b,d)=>{const f=new V(c(a),b>>>U,d>>>U);return e(f)});b.wbg.__wbg_set_a47bac70306a19a7=((a,b,d)=>{c(a).set(c(b),d>>>U)});b.wbg.__wbg_length_c20a40f15020d68a=(a=>{const b=c(a).length;return b});b.wbg.__wbg_buffer_dd7f74bc60f1faab=(a=>{const b=c(a).buffer;return e(b)});b.wbg.__wbg_byteLength_58f7b4fab1919d44=(a=>{const b=c(a).byteLength;return b});b.wbg.__wbg_byteOffset_81d60f7392524f62=(a=>{const b=c(a).byteOffset;return b});b.wbg.__wbg_set_1f9b04f170055d33=function(){return z(((a,b,d)=>{const e=a0.set(c(a),c(b),c(d));return e}),arguments)};b.wbg.__wbindgen_debug_string=((b,d)=>{const e=l(c(d));const f=p(e,a.__wbindgen_export_0,a.__wbindgen_export_1);const g=m;r()[b/$+ Q]=g;r()[b/$+ U]=f});b.wbg.__wbindgen_throw=((a,b)=>{throw new T(k(a,b))});b.wbg.__wbindgen_memory=(()=>{const b=a.memory;return e(b)});b.wbg.__wbindgen_closure_wrapper548=((a,b,c)=>{const d=t(a,b,300,u);return e(d)});b.wbg.__wbindgen_closure_wrapper1609=((a,b,c)=>{const d=t(a,b,662,v);return e(d)});b.wbg.__wbindgen_closure_wrapper3158=((a,b,c)=>{const d=t(a,b,692,w);return e(d)});return b});var f=(a=>{if(a<132)return;b[a]=d;d=a});var t=((b,c,d,e)=>{const f={a:b,b:c,cnt:Q,dtor:d};const g=(...b)=>{f.cnt++;const c=f.a;f.a=U;try{return e(c,f.b,...b)}finally{if(--f.cnt===U){a.__wbindgen_export_2.get(f.dtor)(c,f.b);s.unregister(f)}else{f.a=c}}};g.original=f;s.register(g,f,f);return g});var p=((a,b,c)=>{if(c===O){const c=n.encode(a);const d=b(c.length,Q)>>>U;j().subarray(d,d+ c.length).set(c);m=c.length;return d};let d=a.length;let e=b(d,Q)>>>U;const f=j();let g=U;for(;g<d;g++){const b=a.charCodeAt(g);if(b>127)break;f[e+ g]=b};if(g!==d){if(g!==U){a=a.slice(g)};e=c(e,d,d=g+ a.length*3,Q)>>>U;const b=j().subarray(e+ g,e+ d);const f=o(a,b);g+=f.written;e=c(e,d,g,Q)>>>U};m=g;return e});var j=(()=>{if(i===P||i.byteLength===U){i=new V(a.memory.buffer)};return i});var k=((a,b)=>{a=a>>>U;return h.decode(j().subarray(a,a+ b))});var w=((b,c,d)=>{a.__wbindgen_export_5(b,c,e(d))});let a;const b=new N(128).fill(O);b.push(O,P,!0,!1);let d=b.length;const h=typeof TextDecoder!==R?new TextDecoder(S,{ignoreBOM:!0,fatal:!0}):{decode:()=>{throw T(`TextDecoder not available`)}};if(typeof TextDecoder!==R){h.decode()};let i=P;let m=U;const n=typeof TextEncoder!==R?new TextEncoder(S):{encode:()=>{throw T(`TextEncoder not available`)}};const o=typeof n.encodeInto===X?((a,b)=>n.encodeInto(a,b)):((a,b)=>{const c=n.encode(a);b.set(c);return {read:a.length,written:c.length}});let q=P;const s=typeof Z===R?{register:()=>{},unregister:()=>{}}:new Z(b=>{a.__wbindgen_export_2.get(b.dtor)(b.a,b.b)});const B=typeof Z===R?{register:()=>{},unregister:()=>{}}:new Z(b=>a.__wbg_intounderlyingbytesource_free(b>>>U));class C{__destroy_into_raw(){const a=this.__wbg_ptr;this.__wbg_ptr=U;B.unregister(this);return a}free(){const b=this.__destroy_into_raw();a.__wbg_intounderlyingbytesource_free(b)}type(){try{const e=a.__wbindgen_add_to_stack_pointer(-_);a.intounderlyingbytesource_type(e,this.__wbg_ptr);var b=r()[e/$+ U];var c=r()[e/$+ Q];var d=x(b,c);if(b!==U){a.__wbindgen_export_6(b,c,Q)};return d}finally{a.__wbindgen_add_to_stack_pointer(_)}}autoAllocateChunkSize(){const b=a.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);return b>>>U}start(b){a.intounderlyingbytesource_start(this.__wbg_ptr,e(b))}pull(b){const c=a.intounderlyingbytesource_pull(this.__wbg_ptr,e(b));return g(c)}cancel(){const b=this.__destroy_into_raw();a.intounderlyingbytesource_cancel(b)}}const D=typeof Z===R?{register:()=>{},unregister:()=>{}}:new Z(b=>a.__wbg_intounderlyingsink_free(b>>>U));class E{__destroy_into_raw(){const a=this.__wbg_ptr;this.__wbg_ptr=U;D.unregister(this);return a}free(){const b=this.__destroy_into_raw();a.__wbg_intounderlyingsink_free(b)}write(b){const c=a.intounderlyingsink_write(this.__wbg_ptr,e(b));return g(c)}close(){const b=this.__destroy_into_raw();const c=a.intounderlyingsink_close(b);return g(c)}abort(b){const c=this.__destroy_into_raw();const d=a.intounderlyingsink_abort(c,e(b));return g(d)}}const F=typeof Z===R?{register:()=>{},unregister:()=>{}}:new Z(b=>a.__wbg_intounderlyingsource_free(b>>>U));class G{__destroy_into_raw(){const a=this.__wbg_ptr;this.__wbg_ptr=U;F.unregister(this);return a}free(){const b=this.__destroy_into_raw();a.__wbg_intounderlyingsource_free(b)}pull(b){const c=a.intounderlyingsource_pull(this.__wbg_ptr,e(b));return g(c)}cancel(){const b=this.__destroy_into_raw();a.intounderlyingsource_cancel(b)}}export default M;export{C as IntoUnderlyingByteSource,E as IntoUnderlyingSink,G as IntoUnderlyingSource,L as initSync}
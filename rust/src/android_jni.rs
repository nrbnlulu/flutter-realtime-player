//! GStreamer's standard Android Java runtime providers. Flutter registers the
//! Java plugin before the Rust registrar, so MediaCodec can initialize with the
//! application's real class loader instead of probing private ART entry points.
use jni_sys::{jboolean, jclass, jobject, JNIEnv, JavaVM, JNI_FALSE, JNI_OK, JNI_TRUE};
use std::{ffi::c_void, sync::OnceLock};

struct JavaContext {
    vm: usize,
    loader: usize,
}

// Both values are process-scoped. The loader is a JNI global reference, which
// can be used from any attached thread. Retain it for GStreamer's lifetime.
static JAVA_CONTEXT: OnceLock<JavaContext> = OnceLock::new();

#[no_mangle]
pub unsafe extern "system" fn Java_dev_onetboss_realtimeplayer_FlutterRealtimePlayerPlugin_nativeInitialize(
    env: *mut JNIEnv,
    _class: jclass,
    loader: jobject,
) -> jboolean {
    if JAVA_CONTEXT.get().is_some() {
        return JNI_TRUE;
    }
    if env.is_null() || loader.is_null() {
        return JNI_FALSE;
    }
    // SAFETY: the JVM invokes this method with a valid current-thread JNIEnv
    // and local loader reference. Only the global reference escapes the call.
    let functions = &**env;
    let mut vm: *mut JavaVM = std::ptr::null_mut();
    if functions.GetJavaVM.unwrap()(env, &mut vm) != JNI_OK || vm.is_null() {
        return JNI_FALSE;
    }
    let global_loader = functions.NewGlobalRef.unwrap()(env, loader);
    if global_loader.is_null() {
        return JNI_FALSE;
    }
    if JAVA_CONTEXT
        .set(JavaContext {
            vm: vm as usize,
            loader: global_loader as usize,
        })
        .is_err()
    {
        functions.DeleteGlobalRef.unwrap()(env, global_loader);
    }
    JNI_TRUE
}

#[no_mangle]
pub extern "C" fn gst_android_get_java_vm() -> *mut c_void {
    JAVA_CONTEXT
        .get()
        .map_or(std::ptr::null_mut(), |context| context.vm as *mut c_void)
}

#[no_mangle]
pub extern "C" fn gst_android_get_application_class_loader() -> *mut c_void {
    JAVA_CONTEXT.get().map_or(std::ptr::null_mut(), |context| {
        context.loader as *mut c_void
    })
}

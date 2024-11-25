#[no_mangle]
pub unsafe extern "system" fn Java_top_fifthlight_renderer_Native_sendModelDataAction(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    callback_pointer: jni::sys::jlong,
    data: jni::sys::jbyteArray,
) {
    use jni::objects::JByteArray;
    use renderer::gui::GuiAction;
    use std::sync::mpsc;

    let data = env
        .convert_byte_array(JByteArray::from_raw(data))
        .expect("Failed to read model data");

    let tx = unsafe { Box::from_raw(callback_pointer as *mut mpsc::Sender<GuiAction>) };
    let _ = tx.send(GuiAction::LoadGltfData(None, data));
}

package top.fifthlight.renderer

object Native {
    @JvmStatic
    external fun sendModelDataAction(callbackPointer: Long, data: ByteArray)
}
import Flutter
import VideoToolbox

enum CapabilitiesChannel {
  private static let channelName = "io.github.sndnv.shadowmask/capabilities"

  static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: channelName,
      binaryMessenger: registrar.messenger()
    )
    channel.setMethodCallHandler { call, result in
      switch call.method {
      case "decoding":
        result(decoding())
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  private static func decoding() -> [String: Any] {
    var video: [[String: Any]] = [
      entry("h264", kCMVideoCodecType_H264, 8),
      entry("hevc", kCMVideoCodecType_HEVC, 10),
    ]
    if #available(iOS 16.0, *) {
      video.append(entry("av1", kCMVideoCodecType_AV1, 10))
    }
    return ["video": video]
  }

  private static func entry(
    _ codec: String,
    _ type: CMVideoCodecType,
    _ maxBitDepth: Int
  ) -> [String: Any] {
    return [
      "codec": codec,
      "max_bit_depth": maxBitDepth,
      "smooth": VTIsHardwareDecodeSupported(type),
    ]
  }
}

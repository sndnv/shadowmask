package io.github.sndnv.shadowmask

import android.media.MediaCodecInfo
import android.media.MediaCodecInfo.CodecCapabilities
import android.media.MediaCodecInfo.CodecProfileLevel
import android.media.MediaCodecList
import android.os.Build
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

object CapabilitiesChannel {
    private const val NAME = "io.github.sndnv.shadowmask/capabilities"

    private val TEN_BIT_PROFILES = setOf(
        CodecProfileLevel.AVCProfileHigh10,
        CodecProfileLevel.HEVCProfileMain10,
        CodecProfileLevel.HEVCProfileMain10HDR10,
        CodecProfileLevel.HEVCProfileMain10HDR10Plus,
        CodecProfileLevel.VP9Profile2,
        CodecProfileLevel.VP9Profile3,
        CodecProfileLevel.VP9Profile2HDR,
        CodecProfileLevel.VP9Profile3HDR,
        CodecProfileLevel.VP9Profile2HDR10Plus,
        CodecProfileLevel.VP9Profile3HDR10Plus,
        CodecProfileLevel.AV1ProfileMain10,
        CodecProfileLevel.AV1ProfileMain10HDR10,
        CodecProfileLevel.AV1ProfileMain10HDR10Plus,
    )

    private val HDR10_PROFILES = setOf(
        CodecProfileLevel.HEVCProfileMain10HDR10,
        CodecProfileLevel.VP9Profile2HDR,
        CodecProfileLevel.VP9Profile3HDR,
        CodecProfileLevel.AV1ProfileMain10HDR10,
    )

    private val HDR10_PLUS_PROFILES = setOf(
        CodecProfileLevel.HEVCProfileMain10HDR10Plus,
        CodecProfileLevel.VP9Profile2HDR10Plus,
        CodecProfileLevel.VP9Profile3HDR10Plus,
        CodecProfileLevel.AV1ProfileMain10HDR10Plus,
    )

    private val VIDEO_CODECS = mapOf(
        "video/avc" to "h264",
        "video/hevc" to "hevc",
        "video/x-vnd.on2.vp9" to "vp9",
        "video/av01" to "av1",
    )

    private val AUDIO_CODECS = mapOf(
        "audio/mp4a-latm" to "aac",
        "audio/ac3" to "ac3",
        "audio/eac3" to "eac3",
        "audio/vnd.dts" to "dts",
        "audio/opus" to "opus",
        "audio/flac" to "flac",
        "audio/vorbis" to "vorbis",
        "audio/mpeg" to "mp3",
    )

    private data class Video(val maxBitDepth: Int, val smooth: Boolean)

    private data class Ceiling(val width: Int, val height: Int, val frameRate: Int)

    fun register(engine: FlutterEngine) {
        MethodChannel(engine.dartExecutor.binaryMessenger, NAME)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "decoding" -> result.success(decoding())
                    else -> result.notImplemented()
                }
            }
    }

    private fun decoding(): Map<String, Any> {
        val video = mutableMapOf<String, Video>()
        val audio = mutableMapOf<String, Int>()
        val hdr = mutableSetOf<String>()
        var ceiling: Ceiling? = null
        var tenBitHardware = false

        for (info in codecInfos()) {
            if (info.isEncoder) {
                continue
            }
            val hardware = isHardware(info)
            for (type in info.supportedTypes) {
                val mime = type.lowercase()
                val caps = capabilitiesFor(info, type) ?: continue
                val videoCodec = VIDEO_CODECS[mime]
                if (videoCodec != null) {
                    val depth = bitDepth(caps)
                    val known = video[videoCodec]
                    video[videoCodec] = Video(
                        maxBitDepth = maxOf(known?.maxBitDepth ?: 0, depth),
                        smooth = (known?.smooth ?: false) || hardware,
                    )
                    if (hardware) {
                        hdr += hdrFormats(caps)
                        tenBitHardware = tenBitHardware || depth >= 10
                        ceiling = widest(ceiling, ceilingOf(caps))
                    }
                    continue
                }
                val audioCodec = AUDIO_CODECS[mime] ?: continue
                val channels = caps.audioCapabilities?.maxInputChannelCount ?: 2
                audio[audioCodec] = maxOf(audio[audioCodec] ?: 0, channels)
            }
        }

        if (tenBitHardware) {
            hdr += "hlg"
        }

        val report = mutableMapOf<String, Any>(
            "video" to video.map { (codec, entry) ->
                mapOf(
                    "codec" to codec,
                    "max_bit_depth" to entry.maxBitDepth,
                    "smooth" to entry.smooth,
                )
            },
            "audio" to audio.map { (codec, channels) ->
                mapOf("codec" to codec, "max_channels" to channels)
            },
            "hdr" to hdr.toList(),
        )
        ceiling?.let {
            report["max_width"] = it.width
            report["max_height"] = it.height
            if (it.frameRate > 0) {
                report["max_frame_rate"] = it.frameRate
            }
        }
        return report
    }

    private fun codecInfos(): Array<MediaCodecInfo> =
        runCatching { MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos }
            .getOrDefault(emptyArray())

    private fun capabilitiesFor(info: MediaCodecInfo, type: String): CodecCapabilities? =
        runCatching { info.getCapabilitiesForType(type) }.getOrNull()

    private fun isHardware(info: MediaCodecInfo): Boolean =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            info.isHardwareAccelerated
        } else {
            val name = info.name.lowercase()
            !name.startsWith("omx.google.") && !name.startsWith("c2.android.")
        }

    private fun bitDepth(caps: CodecCapabilities): Int =
        if (caps.profileLevels.any { it.profile in TEN_BIT_PROFILES }) 10 else 8

    private fun hdrFormats(caps: CodecCapabilities): Set<String> {
        val formats = mutableSetOf<String>()
        for (level in caps.profileLevels) {
            if (level.profile in HDR10_PROFILES) {
                formats += "hdr10"
            }
            if (level.profile in HDR10_PLUS_PROFILES) {
                formats += "hdr10plus"
            }
        }
        return formats
    }

    private fun ceilingOf(caps: CodecCapabilities): Ceiling? {
        val video = caps.videoCapabilities ?: return null
        val width = runCatching { video.supportedWidths.upper }.getOrNull() ?: return null
        val height = runCatching { video.supportedHeights.upper }.getOrNull() ?: return null
        val frameRate = runCatching {
            video.getSupportedFrameRatesFor(width, height).upper.toInt()
        }.getOrDefault(0)
        return Ceiling(width, height, frameRate)
    }

    private fun widest(known: Ceiling?, found: Ceiling?): Ceiling? {
        if (found == null) {
            return known
        }
        if (known == null) {
            return found
        }
        return if (found.width.toLong() * found.height > known.width.toLong() * known.height) {
            found
        } else {
            known
        }
    }
}

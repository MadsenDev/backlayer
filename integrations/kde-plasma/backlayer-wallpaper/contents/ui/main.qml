import QtQuick
import QtQuick.Particles
import QtMultimedia
import QtCore
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as Plasma5Support

WallpaperItem {
    id: root

    property string wallpaperKind: ""
    property string wallpaperPath: ""
    property bool configured: false
    property string statusText: "Loading…"
    property var sceneDoc: null
    property string sceneDirPath: ""

    readonly property string configPath: {
        var home = StandardPaths.writableLocation(StandardPaths.HomeLocation).toString().replace(/^file:\/\//, "")
        return home + "/.config/backlayer/config.toml"
    }

    Plasma5Support.DataSource {
        id: configReader
        engine: "executable"
        connectedSources: []
        onNewData: function(sourceName, data) {
            configReader.disconnectSource(sourceName)
            var out = data["stdout"] || ""
            if (out.length > 0) root.parseConfig(out)
            else { root.configured = false; root.statusText = "No Backlayer config found" }
        }
    }

    Plasma5Support.DataSource {
        id: sceneReader
        engine: "executable"
        connectedSources: []
        onNewData: function(sourceName, data) {
            sceneReader.disconnectSource(sourceName)
            var out = data["stdout"] || ""
            if (out.length > 0) {
                try { root.sceneDoc = JSON.parse(out) }
                catch(e) { root.statusText = "Invalid scene file" }
            }
        }
    }

    Timer {
        interval: 5000
        repeat: true
        running: true
        triggeredOnStart: true
        onTriggered: configReader.connectSource("cat '" + root.configPath + "' 2>/dev/null")
    }

    function parseConfig(toml) {
        var kindMatch = toml.match(/kind\s*=\s*"(image|video|shader|scene|web)"/)
        var entrypointMatch = toml.match(/entrypoint\s*=\s*"([^"]+)"/)
        if (!kindMatch || !entrypointMatch) {
            root.configured = false; root.statusText = "No wallpaper assigned"; return
        }
        var newKind = kindMatch[1]
        var newPath = "file://" + entrypointMatch[1]
        root.wallpaperKind = newKind
        if (newPath !== root.wallpaperPath) {
            root.wallpaperPath = newPath
            root.sceneDoc = null
            if (newKind === "scene") {
                var plain = entrypointMatch[1]
                root.sceneDirPath = plain.replace(/\/[^\/]+$/, "/")
                sceneReader.connectSource("cat '" + plain + "' 2>/dev/null")
            }
        }
        root.configured = true
        root.statusText = ""
    }

    // Always-visible dark background
    Rectangle {
        anchors.fill: parent
        color: "#0b1020"
    }

    // ── Image ─────────────────────────────────────────────────────────────────
    Image {
        anchors.fill: parent
        visible: root.wallpaperKind === "image"
        source: root.wallpaperKind === "image" ? root.wallpaperPath : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
    }

    // ── Video ─────────────────────────────────────────────────────────────────
    MediaPlayer {
        id: player
        source: root.wallpaperKind === "video" ? root.wallpaperPath : ""
        loops: MediaPlayer.Infinite
        videoOutput: videoOut
        onSourceChanged: if (source !== "") play()
    }
    VideoOutput {
        id: videoOut
        anchors.fill: parent
        visible: root.wallpaperKind === "video"
        fillMode: VideoOutput.PreserveAspectCrop
    }

    // ── Scene ─────────────────────────────────────────────────────────────────
    Item {
        id: sceneLayer
        anchors.fill: parent
        visible: root.wallpaperKind === "scene" && root.sceneDoc !== null

        // Map image key → file:// URL
        property var imageMap: {
            var map = {}
            if (!root.sceneDoc) return map
            for (var i = 0; i < root.sceneDoc.images.length; i++) {
                var img = root.sceneDoc.images[i]
                map[img.key] = "file://" + root.sceneDirPath + img.path
            }
            return map
        }

        property var spriteNodes: root.sceneDoc ? root.sceneDoc.nodes.filter(function(n) {
            return n.kind === "sprite" && n.enabled
        }) : []
        property var glowNodes: root.sceneDoc ? root.sceneDoc.nodes.filter(function(n) {
            return n.kind === "effect" && n.enabled && n.effect === "glow"
        }) : []
        property var emitterNodes: root.sceneDoc ? root.sceneDoc.nodes.filter(function(n) {
            return n.kind === "emitter" && n.enabled
        }) : []

        // Sprite images
        Repeater {
            model: sceneLayer.spriteNodes
            Image {
                required property var modelData
                anchors.fill: parent
                source: sceneLayer.imageMap[modelData.image_key] || ""
                fillMode: modelData.fit === "cover" ? Image.PreserveAspectCrop : Image.PreserveAspectFit
                opacity: modelData.opacity !== undefined ? modelData.opacity : 1.0
                asynchronous: true
            }
        }

        // Glow effects — animated tinted overlay
        Repeater {
            model: sceneLayer.glowNodes
            Item {
                id: glowItem
                required property var modelData
                anchors.fill: parent

                property real peakOpacity: (modelData.opacity || 0.5) * (modelData.intensity || 0.5) * 0.35
                property real cycleDuration: Math.max(400, Math.round(1400 / (modelData.speed || 0.85)))

                Rectangle {
                    anchors.fill: parent
                    color: glowItem.modelData.color_hex || "#8888ff"
                    opacity: glowItem.peakOpacity
                }

                SequentialAnimation on opacity {
                    loops: Animation.Infinite
                    NumberAnimation { from: 0.6; to: 1.0; duration: glowItem.cycleDuration; easing.type: Easing.InOutSine }
                    NumberAnimation { from: 1.0; to: 0.6; duration: glowItem.cycleDuration; easing.type: Easing.InOutSine }
                }
            }
        }

        // Particle emitters
        Repeater {
            model: sceneLayer.emitterNodes
            Item {
                id: emitterItem
                required property var modelData
                anchors.fill: parent

                property string pColor: modelData.color_hex || "#ff9452"
                property real pOpacity: modelData.opacity || 0.6

                ParticleSystem {
                    id: particleSys
                    anchors.fill: parent
                }

                Emitter {
                    system: particleSys
                    // origin_x/y is the center of the spawn area, so offset by half the emitter size
                    width: (emitterItem.modelData.region_radius || 0.05) * 2.0 * parent.width
                    height: (emitterItem.modelData.region_radius || 0.05) * 2.0 * parent.height
                    x: (emitterItem.modelData.origin_x || 0.5) * parent.width - width / 2
                    y: (emitterItem.modelData.origin_y || 0.5) * parent.height - height / 2
                    emitRate: Math.min(emitterItem.modelData.emission_rate || 10, 60)
                    lifeSpan: ((emitterItem.modelData.min_life || 2.0) + (emitterItem.modelData.max_life || 5.0)) / 2.0 * 1000
                    lifeSpanVariation: Math.abs((emitterItem.modelData.max_life || 5.0) - (emitterItem.modelData.min_life || 2.0)) / 2.0 * 1000
                    size: emitterItem.modelData.size || 4
                    sizeVariation: (emitterItem.modelData.size || 4) * 0.4
                    velocity: AngleDirection {
                        // direction_deg uses y-down screen coords (same as Qt Quick) — just normalize negatives
                        angle: ((emitterItem.modelData.direction_deg || -90) % 360 + 360) % 360
                        // spread is full angular width; angleVariation is the half-angle, so divide by 2
                        angleVariation: (emitterItem.modelData.spread || 30) / 2
                        magnitude: ((emitterItem.modelData.min_speed || 50) + (emitterItem.modelData.max_speed || 100)) / 2.0
                        magnitudeVariation: Math.abs((emitterItem.modelData.max_speed || 100) - (emitterItem.modelData.min_speed || 50)) / 2.0
                    }
                }

                Gravity {
                    system: particleSys
                    magnitude: Math.abs(emitterItem.modelData.gravity_y || 18)
                    angle: (emitterItem.modelData.gravity_y || 0) < 0 ? 270 : 90
                }

                ItemParticle {
                    system: particleSys
                    delegate: Rectangle {
                        width: 4; height: 4; radius: 2
                        color: emitterItem.pColor
                        opacity: emitterItem.pOpacity
                    }
                }
            }
        }
    }

    // ── Status / unsupported ──────────────────────────────────────────────────
    Text {
        anchors.centerIn: parent
        visible: root.statusText !== ""
            || (root.configured
                && root.wallpaperKind !== "image"
                && root.wallpaperKind !== "video"
                && root.wallpaperKind !== "scene")
        text: root.statusText !== "" ? root.statusText
            : "Backlayer — " + root.wallpaperKind + " wallpapers not yet renderable in KDE bridge"
        color: "#9db4cc"
        font.pixelSize: Math.max(12, root.width * 0.012)
        horizontalAlignment: Text.AlignHCenter
    }

    Component.onCompleted: {
        console.log("[BacklayerWallpaper] loaded", root.width, root.height, "config:", root.configPath)
    }
}

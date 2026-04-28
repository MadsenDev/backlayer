import QtQuick
import org.kde.plasma.wallpaper 2.0

WallpaperItem {
    id: root

    Rectangle {
        anchors.fill: parent
        color: "#0b1020"
    }

    Rectangle {
        id: glow
        width: Math.max(120, root.width * 0.18)
        height: width
        radius: width / 2
        opacity: 0.55
        color: "#4f7cff"
        y: root.height * 0.2

        SequentialAnimation on x {
            loops: Animation.Infinite
            NumberAnimation {
                from: -glow.width * 0.35
                to: Math.max(root.width - glow.width * 0.65, 1)
                duration: 4600
                easing.type: Easing.InOutQuad
            }
            NumberAnimation {
                from: Math.max(root.width - glow.width * 0.65, 1)
                to: -glow.width * 0.35
                duration: 4600
                easing.type: Easing.InOutQuad
            }
        }
    }

    Rectangle {
        id: pulse
        width: Math.max(84, root.width * 0.1)
        height: width
        radius: width / 2
        color: "#8bd7ff"
        opacity: 0.32
        anchors.centerIn: parent

        SequentialAnimation on scale {
            loops: Animation.Infinite
            NumberAnimation { from: 0.8; to: 1.25; duration: 1800; easing.type: Easing.InOutQuad }
            NumberAnimation { from: 1.25; to: 0.8; duration: 1800; easing.type: Easing.InOutQuad }
        }
    }

    Column {
        anchors.centerIn: parent
        spacing: 10

        Text {
            text: "Backlayer"
            color: "#f8fafc"
            font.pixelSize: Math.max(28, root.width * 0.032)
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
            anchors.horizontalCenter: parent.horizontalCenter
        }

        Text {
            text: "KDE Plasma bridge foundation"
            color: "#9db4cc"
            font.pixelSize: Math.max(12, root.width * 0.012)
            horizontalAlignment: Text.AlignHCenter
            anchors.horizontalCenter: parent.horizontalCenter
        }
    }

    Text {
        anchors {
            left: parent.left
            bottom: parent.bottom
            leftMargin: 16
            bottomMargin: 12
        }
        text: "Animated placeholder active"
        color: "#7f8ea3"
        font.pixelSize: Math.max(10, root.width * 0.01)
    }

    Component.onCompleted: {
        console.log("[BacklayerWallpaper] loaded", root.width, root.height)
    }
}

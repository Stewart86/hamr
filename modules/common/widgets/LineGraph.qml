import qs.modules.common
import QtQuick

Canvas {
    id: root
    property var data: []
    property real min: 0
    property real max: 0
    property int size: 40

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    onDataChanged: requestPaint()
    onMinChanged: requestPaint()
    onMaxChanged: requestPaint()
    onSizeChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d");
        ctx.clearRect(0, 0, width, height);

        if (data.length < 2) return;

        const padding = 2;
        const graphWidth = width - padding * 2;
        const graphHeight = height - padding * 2;

        let minValue = min;
        let maxValue = max;

        if (minValue === 0 && maxValue === 0) {
            minValue = Math.min(...data);
            maxValue = Math.max(...data);
        }

        if (minValue === maxValue) {
            maxValue = minValue + 1;
        }

        const range = maxValue - minValue;

        const points = data.map((value, index) => ({
            x: padding + (index / (data.length - 1)) * graphWidth,
            y: height - padding - ((value - minValue) / range) * graphHeight
        }));

        ctx.strokeStyle = Appearance?.colors.colPrimary ?? "#685496";
        ctx.lineWidth = 2;
        ctx.lineCap = "round";
        ctx.lineJoin = "round";
        ctx.beginPath();
        ctx.moveTo(points[0].x, points[0].y);

        for (let i = 0; i < points.length - 1; i++) {
            const xc = (points[i].x + points[i + 1].x) / 2;
            const yc = (points[i].y + points[i + 1].y) / 2;
            ctx.quadraticCurveTo(points[i].x, points[i].y, xc, yc);
        }

        const lastPoint = points[points.length - 1];
        ctx.quadraticCurveTo(
            points[points.length - 2].x,
            points[points.length - 2].y,
            lastPoint.x,
            lastPoint.y
        );

        ctx.stroke();
    }
}

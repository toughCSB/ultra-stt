package com.ultrastt.androidvoice;

final class RadialGeometry {
    static final double RADIUS = 108;
    static final double DEAD_ZONE = 46;
    static final double MAX_DISTANCE = 166;

    static double x(int index) {
        return RADIUS * Math.cos(Math.toRadians(270 - index * 45));
    }

    static double x(int index, boolean opensRight) {
        return opensRight ? -x(index) : x(index);
    }

    static int pick(double deltaX, double deltaY, boolean opensRight) {
        return pick(opensRight ? -deltaX : deltaX, deltaY);
    }

    static double y(int index) {
        return RADIUS * Math.sin(Math.toRadians(270 - index * 45));
    }

    static int pick(double deltaX, double deltaY) {
        if (!Double.isFinite(deltaX) || !Double.isFinite(deltaY)) {
            return -1;
        }
        double distance = Math.hypot(deltaX, deltaY);
        if (deltaX > 1 || distance < DEAD_ZONE || distance > MAX_DISTANCE) {
            return -1;
        }
        double angle = Math.toDegrees(Math.atan2(deltaY, deltaX));
        if (angle < 0) {
            angle += 360;
        }
        if (angle < 90 || angle > 270) {
            return -1;
        }
        return Math.max(0, Math.min(4, (int) Math.round((270 - angle) / 45)));
    }

    private RadialGeometry() {}
}

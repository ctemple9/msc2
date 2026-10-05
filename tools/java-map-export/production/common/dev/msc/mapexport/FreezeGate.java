package dev.msc.mapexport;

/** Simulation guard applies only to the independently verified private server. */
public final class FreezeGate {
    private static volatile Object frozen;
    private static volatile boolean observed;
    private FreezeGate() {}
    public static void clear() { frozen=null; observed=false; }
    public static void freeze(Object server) { observed=false; frozen=server; }
    public static boolean skip(Object server) {
        if(server != frozen)return false;
        observed=true;return true;
    }
    public static boolean active(Object server) {return server!=null&&frozen==server;}
    public static boolean observed(Object server) {return frozen==server&&observed;}
}

// P7.27's fake server jar: stands in for a real Vanilla/Paper/Purpur/Fabric
// server jar (`-jar <jar> --nogui`) so the portable six-family smoke never
// touches a real provider or a real Minecraft download. Prints the exact
// OS-level command line it was launched with (`LAUNCH_ARGV:`) so the smoke
// can confirm the real launch shape from the server's own console output --
// the same signal it also relies on for Forge/NeoForge -- rather than
// inspecting the process table, which isn't portable across the platforms
// this smoke eventually runs on (P7.29).
import java.io.BufferedReader;
import java.io.InputStreamReader;

public class FakeServer {
    public static void main(String[] args) throws Exception {
        System.out.println("LAUNCH_ARGV:" + ProcessHandle.current().info().commandLine().orElse("?"));
        System.out.flush();
        Thread.sleep(200);
        System.out.println("Done (0.001s)! For help, type \"help\"");
        System.out.flush();

        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            if (line.equals("stop")) {
                System.out.println("Stopping fake server");
                System.out.flush();
                return;
            }
        }
    }
}

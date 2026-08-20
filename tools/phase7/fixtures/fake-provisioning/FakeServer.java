// P7.27's fake server jar: stands in for a real Vanilla/Paper/Purpur/Fabric
// server jar (`-jar <jar> --nogui`) so the portable six-family smoke never
// touches a real provider or a real Minecraft download. Prints the launch
// shape it was actually started with (`LAUNCH_ARGV:`) so the smoke can
// confirm it from the server's own console output -- rather than
// inspecting the process table, which isn't portable across the platforms
// this smoke eventually runs on (P7.29).
//
// `sun.java.command`, not `ProcessHandle.current().info().commandLine()`:
// P7.29's own Windows CI leg found the latter always empty there --
// JDK-8176725, `ProcessHandle.Info`'s Windows implementation never
// populates `arguments`, so `commandLine()` (which needs both `command()`
// and `arguments()` present) is never present either. `sun.java.command`
// is set directly from the JVM's own parsed argv at startup, not an OS
// re-query, so it works on every platform -- but by the time it's built,
// the launcher has already stripped `-jar` and expanded `@<args-file>`
// into its own contents, so it reads `"<jar> <args>"` for a jar launch and
// `"<MainClass> <args>"` for an args-file launch, never literally `-jar`
// or `@` (verified locally against a real `-jar`/`@file` launch of a
// throwaway probe jar). `assert_launch_argv_shape` below checks the first
// token's `.jar` suffix instead of the old `-jar`/`@` substrings.
import java.io.BufferedReader;
import java.io.InputStreamReader;

public class FakeServer {
    public static void main(String[] args) throws Exception {
        String command = System.getProperty("sun.java.command");
        System.out.println("LAUNCH_ARGV:" + (command != null ? command : "?"));
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

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
//
// P7.37: two file-based control signals, read relative to the process's
// own working directory -- which is always the server's own directory
// (`ProcessSpawnRequest::working_directory`, confirmed against
// `crates/msc-agent/src/routes/lifecycle.rs`), so the smoke script can
// drop one in before `server start` without any production wiring:
//
//   smoke-plugin-failure.txt  -- one plugin name per line. Printed as a
//     Paper "Error occurred while enabling <name> v1.0" line *before*
//     "Done", matching a real Paper plugin failing to enable during
//     startup while the server still finishes booting (a soft failure
//     `analyze_paper_plugins` picks up).
//   smoke-mod-crash.txt -- one raw console line. Printed, then the
//     process exits nonzero *without* ever printing "Done", matching a
//     modded server that dies before reaching ready (a hard failure
//     `diagnose_unexpected_stop` picks up). Mutually exclusive with the
//     plugin-failure file in practice (no fixture uses both).
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

public class FakeServer {
    public static void main(String[] args) throws Exception {
        String command = System.getProperty("sun.java.command");
        System.out.println("LAUNCH_ARGV:" + (command != null ? command : "?"));
        System.out.flush();
        Thread.sleep(200);

        Path crashSignal = Path.of("smoke-mod-crash.txt");
        if (Files.exists(crashSignal)) {
            for (String line : readLines(crashSignal)) {
                System.out.println(line);
            }
            System.out.flush();
            System.exit(1);
        }

        Path pluginFailureSignal = Path.of("smoke-plugin-failure.txt");
        if (Files.exists(pluginFailureSignal)) {
            for (String name : readLines(pluginFailureSignal)) {
                System.out.println("[Server thread/ERROR]: Error occurred while enabling "
                    + name + " v1.0 (Is it up to date?)");
            }
            System.out.flush();
        }

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

    private static List<String> readLines(Path path) throws IOException {
        return Files.readAllLines(path).stream().filter(l -> !l.isBlank()).toList();
    }
}

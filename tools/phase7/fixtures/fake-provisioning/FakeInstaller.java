// P7.27's fake Forge/NeoForge loader installer. `fake_provider_server.py`
// builds one jar per download request from this precompiled class plus a
// freshly written `/install-target.properties` resource naming which
// family/version this particular download is for -- the same "one jar per
// version" shape a real installer download has, without a real network
// fetch.
//
// Dual mode, dispatched on `args[0]`, mirroring exactly how
// `run_loader_installer` (`crates/msc-infrastructure/src/
// loader_installer.rs`) invokes a real installer and how the resulting
// args file then re-invokes whatever it names as the boot-time main class:
//   `java -jar <installer> --installServer`   -> install mode
//   `java @<args-file> nogui`                 -> boot mode (args[0] is
//                                                 "--launchTarget", not
//                                                 "--installServer")
public class FakeInstaller {
    public static void main(String[] args) throws Exception {
        // sun.java.command, not ProcessHandle -- see FakeServer.java's own
        // doc comment for why (JDK-8176725, found by P7.29's Windows leg).
        String command = System.getProperty("sun.java.command");
        System.out.println("LAUNCH_ARGV:" + (command != null ? command : "?"));
        System.out.flush();

        boolean installMode = args.length > 0 && args[0].equals("--installServer");
        if (!installMode) {
            bootLoop();
            return;
        }
        install();
    }

    static void bootLoop() throws Exception {
        Thread.sleep(200);
        System.out.println("Done (0.001s)! For help, type \"help\"");
        System.out.flush();

        java.io.BufferedReader reader =
                new java.io.BufferedReader(new java.io.InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            if (line.equals("stop")) {
                System.out.println("Stopping fake server");
                System.out.flush();
                return;
            }
        }
    }

    static void install() throws Exception {
        java.util.Properties p = new java.util.Properties();
        try (java.io.InputStream in =
                FakeInstaller.class.getResourceAsStream("/install-target.properties")) {
            p.load(in);
        }
        String family = p.getProperty("family");
        String controlDir = p.getProperty("control_dir");
        long delayMs = Long.parseLong(p.getProperty("install_delay_ms", "0"));

        System.out.println("[FakeInstaller] installing " + family);
        System.out.flush();
        if (delayMs > 0) {
            Thread.sleep(delayMs);
        }

        java.io.File failMarker = new java.io.File(controlDir, "fail_install");
        if (failMarker.exists()) {
            System.err.println("[FakeInstaller] injected installer failure");
            System.err.flush();
            System.exit(1);
        }

        String baseDir;
        if (family.equals("forge")) {
            baseDir = "libraries/net/minecraftforge/forge/" + p.getProperty("pair");
        } else {
            baseDir = "libraries/net/neoforged/neoforge/" + p.getProperty("version");
        }
        java.io.File dir = new java.io.File(baseDir);
        dir.mkdirs();

        // Copy the installer jar currently executing (this class's own
        // containing jar) into the install directory as the "loader" jar
        // the args file's classpath will point at -- the boot-mode half of
        // this same class is what actually runs when launched that way.
        String selfPath = new java.io.File(FakeInstaller.class.getProtectionDomain()
                .getCodeSource().getLocation().toURI()).getAbsolutePath();
        java.nio.file.Files.copy(
                java.nio.file.Paths.get(selfPath),
                java.nio.file.Paths.get(baseDir, "fake-loader.jar"),
                java.nio.file.StandardCopyOption.REPLACE_EXISTING);

        // The `@<args-file>` JEP 293 shape: one token per line, resolved
        // relative to the process's own working directory (the server
        // directory) exactly like `find_forge_args_file`/
        // `find_neoforge_args_file`'s returned path already assumes.
        try (java.io.PrintWriter w =
                new java.io.PrintWriter(new java.io.FileWriter(new java.io.File(dir, "unix_args.txt")))) {
            w.println("-cp");
            w.println(baseDir + "/fake-loader.jar");
            w.println("FakeInstaller");
            w.println("--launchTarget");
            w.println("forgeserver");
        }

        System.out.println("[FakeInstaller] wrote " + baseDir + "/unix_args.txt");
        System.out.flush();
    }
}

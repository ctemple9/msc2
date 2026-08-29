---
id: handbook.jars-java
kind: handbook
title: JAR Files & Java
category: java-servers
subtitle: "The files that power your Java server and the engine that runs them."
analogy: "A JAR file is a sealed package containing all the server's code. Java is the machine that opens that package and runs it. You need both — the package alone doesn't do anything, and the machine needs a package to run."
relatedIds: [health.java, handbook.first-server]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: jarsJavaContent}
---

**JAR** stands for Java ARchive. It’s a bundle of compiled code — essentially the server software in a single file.

**Java** is the runtime that executes that code. It’s not specific to Minecraft; it’s a general-purpose programming platform that millions of programs use.

The launch command varies by server type:

**Standard servers (Paper/Purpur/Vanilla):**
`java -Xms2G -Xmx4G -jar paper.jar`

**Fabric modded servers** use a launcher JAR the Fabric installer generates:
`java -jar fabric-server-launch.jar`

**NeoForge/Forge modded servers** use a shell script the installer generates. It passes a long @args file to Java with remapping flags and a classpath. MSC reads and runs this script for you — you never have to touch it directly.

Note: JAR files and Java are only needed for Java servers. Bedrock servers use the built-in VM instead.

### Callout: warning

Java must be installed on your Mac before a Java server can start. The app recommends Temurin 21 (from Adoptium). Use Preferences → Check for Java to verify your setup.

### In This App

- Preferences → Java Path: tells the app which java executable to use.
- Manage Servers → Edit: each Java server has its own Paper JAR path (usually paper.jar inside the server folder).
- Details tab: shows a JAR summary — which Paper, Geyser, and Floodgate builds this server is using and when they were last updated.
- Update Paper / Update Geyser / Update Floodgate buttons: one-click updates from your saved templates.

### Callout: pitfall

Common pitfall: if you see "java not found" errors, your Java installation isn't in the expected location. Open Preferences, click Check for Java, and follow the prompts.

### Advanced Details

You can have multiple Java versions installed on your Mac (common with developers). You can point Minecraft Server Controller to a specific binary — for example:
  /Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java

This is useful if you want to test a server on an older Java version, or if you use a tool like SDKMAN to manage multiple JDKs.

Geyser and Floodgate are also JAR files — they live in your server's plugins/ folder and get picked up automatically when the server starts.

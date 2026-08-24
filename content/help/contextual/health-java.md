---
id: health.java
kind: contextual
title: Java runtime
category: health
analogy: Java is the engine that opens and runs a Java server JAR.
relatedIds: [handbook.jars-java, health.last-startup]
source: {path: "crates/msc-application/src/diagnostics.rs", symbol: check_java_runtime}
---
Java servers need a compatible Java executable. The health check reports whether MSC found one and whether its version can be read.

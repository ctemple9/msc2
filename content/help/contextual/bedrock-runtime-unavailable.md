---
id: bedrock.runtime-unavailable
kind: contextual
title: Bedrock runtime unavailable
category: bedrock
analogy: The Bedrock runtime is the engine bay for a Bedrock server; a missing bay cannot be replaced by a Java setting.
relatedIds: [handbook.bedrock, handbook.how-bedrock-runs]
source: {path: "crates/msc-api/src/dto/capabilities.rs", symbol: BedrockRuntimeStateDto::unavailable_error}
---
This host cannot currently run the advertised Bedrock runtime. The reason code explains whether installation, host support, or test-evidence limits apply; do not infer support from the operating system name.

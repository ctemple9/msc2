---
id: handbook.networking-basics
kind: handbook
title: How Servers Connect
category: concepts
subtitle: "Public IPs, private IPs, NAT, and the two ways friends reach your server."
analogy: "The internet is a city. Every building (internet connection) has one street address (public IP). Inside each building are many apartments (devices). Your Mac is one apartment. When a friend wants to visit, they need the building address AND the apartment number — and your building’s front desk (router) needs to be told which apartment to send them to."
relatedIds: [handbook.port-forwarding-duckdns, handbook.playit]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: networkingBasicsContent}
---

**Public IP** is the address your internet provider assigns to your home connection. Everyone outside your home sees this address when they try to reach something on your network. It’s shared by all devices in your house.

**Private IP** is the address your router assigns to each device inside your home — your Mac, your phone, your TV. These addresses (like 192.168.1.x or 10.0.0.x) only exist inside your home network and can’t be reached directly from the internet.

**NAT (Network Address Translation)** is what your router does: it translates between your one public IP and all your private devices. Outbound traffic works automatically — your Mac can reach the internet at any time. Inbound traffic is blocked by default — the router doesn’t know which device an incoming connection is meant for.

This is why you can load any website (your Mac reaches out) but a friend can’t connect to your server without extra setup (they’re trying to reach in).

### Callout: note

LAN players — people on the same Wi-Fi or wired network as your Mac — connect directly using your private IP and need nothing special. The external access problem only affects players outside your home.

### Body

**Two ways to solve this:**

### In This App

- Port Forwarding: tell your router to send traffic on a specific port (25565 TCP for Java, 19132 UDP for Bedrock) to your Mac. Requires router access and a reachable public IP. Free — no external service needed. See the Port Forwarding & DuckDNS topic.
- Playit.gg Tunneling: your Mac connects outbound to Playit.gg’s relay servers. Friends connect to an address Playit.gg gives you. No router access needed, but adds some latency and requires a free Playit.gg account. See the Playit.gg Tunneling topic.

### Callout: tip

Not sure which to use? If you have access to your router and your ISP gives you a standard public IP, port forwarding is simpler long-term. If you’re on a shared network, behind CGNAT (some mobile and cable ISPs), or just don’t want to configure your router, Playit.gg is the better choice.

### Advanced Details

CGNAT (Carrier-Grade NAT) is used by some ISPs — particularly mobile carriers and some cable providers. Under CGNAT, your home doesn’t get its own public IP; you share one with many other customers. Port forwarding is impossible under CGNAT. If you set up port forwarding correctly and it still never works, CGNAT may be the reason.

You can check for CGNAT by comparing the WAN IP shown in your router’s admin panel to the IP shown on a site like whatismyip.com. If they’re different, you’re behind CGNAT.

Playit.gg and all other tunneling solutions work under CGNAT because they rely only on outbound connections from your Mac.

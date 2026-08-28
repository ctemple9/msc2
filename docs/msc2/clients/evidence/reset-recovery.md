# Reset and first-launch recovery evidence

**Step:** P12.19d — Resume first launch and prove reset recovery end to end

This worksheet separates deterministic checks from Cameron's real host
walkthroughs. The automated checks prove that the shared client keeps reset
local, carries a rotated remote host identity through pairing, and never calls
server creation as a side effect of recovery. The real walkthroughs below are
the evidence for service behavior, filesystem preservation, and visual flow.

## Automated evidence

Run the exact P12.19d Verify command from `docs/msc2/rolling-plan.md`.

Expected coverage:

- local missing-agent recovery shows **Install and Continue**;
- local stopped-agent recovery shows **Start and Continue**;
- an incompatible agent offers **Repair service**;
- remote recovery accepts a fresh one-use pairing code and replaces the old
  host identity;
- first launch loads host setup, Concept Guide, and onboarding data from the
  agent, with no `/v1/servers/create` call during recovery;
- a browser client reset returns to first launch while the host remains
  unchanged.

## Cameron's real walkthroughs

Record the date, platform, agent host ID before and after, and the observed
result for each row. Do not paste bearer credentials or pairing codes here.

| Walkthrough | Procedure | Evidence to record | Result |
|---|---|---|---|
| Client-only reset | Complete first launch on a client with a configured host. Open Preferences → Reset this client, confirm, then reconnect. | Host configuration and server files unchanged; local host records and onboarding flags cleared; Concept Guide reopens; no server is created. | Pending |
| Remote configuration reset | Pair a second desktop to a remote host. Stop every Minecraft server. Reset the remote host with **Configuration only**. | Existing server folder/worlds/jars/logs remain; old credential is rejected; agent service remains installed; a fresh `msc pairing create` code completes **Pair Again** and reopens host setup. | Pending |
| Remote full reset | On the same remote host with all servers stopped, reset with **Everything**. | Managed server folder is removed; agent service remains installed; old credential is rejected; fresh pairing is required and succeeds; no server is created. | Pending |
| Local full reset | On the desktop host with all servers stopped, reset with **Everything**. | Host state and managed server folder are removed; the desktop uninstalls its local service; the agent screen shows **Install and Continue**; install bootstraps a new credential and opens host setup. | Pending |
| Running-server refusal | Start a managed server, then attempt both reset modes. | Both reset confirmations are refused with `409 server_running`; files, service, credentials, and host identity remain unchanged. | Pending |
| Confirmation boundary | Try a wrong host ID, a stale host ID, and the exact `RESET <current-agent-host-id>` value. | Wrong and stale values are rejected; only the exact current identity is accepted. | Pending |

The first-server handoff is verified separately after recovery: walk SetupIntro
→ Concept Guide → guided tour, use the highlighted Add Server action, complete
the wizard, and confirm that the server appears only after the explicit Create
action. Capture one screenshot of the first-launch sequence and one of the
post-create handoff.

// Generated from docs/msc2/api-contract/openapi.json. Do not edit by hand.
// Contract SHA-256: eaf690f970786afc40ee472316cd2b48159f19b2b41c97395158672702a2b508

export interface paths {
  '/v1/active-server': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Select which registered server is active */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': {
            serverId: string;
          } & {
            [key: string]: unknown;
          };
        };
      };
      responses: {
        /** @description activated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['SimpleResult'];
          };
        };
        /** @description missing_body / missing_server_id / invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description unknown_server */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/addons': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Installed add-ons (mods/plugins) with update status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Installed add-ons (mods/plugins) with update status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['AddonsResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/allowlist': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the Bedrock allowlist */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current allowlist */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['AllowlistResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Add or remove a Bedrock allowlist entry */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['AllowlistMutationRequestDTO'];
        };
      };
      responses: {
        /** @description Entry added/removed, echoes fresh list */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['AllowlistMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / invalid_action / missing_name */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description not_bedrock */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/auth/browser-sessions': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Exchange a browser pairing code for the current browser session */
    post: operations['exchangeBrowserSession'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/auth/browser-sessions/current': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    post?: never;
    /** Revoke the current browser session */
    delete: operations['logoutBrowserSession'];
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/auth/csrf': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the current browser session's CSRF token */
    get: operations['getCsrfToken'];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/auth/desktop-pairings': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Exchange a desktop pairing code for a host-scoped bearer credential */
    post: operations['exchangeDesktopPairing'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/auth/pairings': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Create a one-use pairing code for a browser or desktop client */
    post: operations['createPairing'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/backups': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Backups available for the active server */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Backups available for the active server */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BackupsResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/backups/config': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get auto-backup configuration */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current config */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BackupConfigResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Update auto-backup configuration */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['BackupConfigUpdateRequestDTO'];
        };
      };
      responses: {
        /** @description Config updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BackupConfigUpdateResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / no_changes */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description update rejected */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/backups/delete': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Delete a backup by id */
    post: operations['deleteBackup'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/backups/now': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Start an immediate backup */
    post: operations['createBackupNow'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/backups/restore': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Restore a backup by id (filename) */
    post: operations['restoreBackup'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/auth-prompt': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get pending MCXboxBroadcast auth prompt */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current prompt state */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastAuthPromptDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/auth-prompt/dismiss': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Dismiss the pending auth prompt */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description result: dismissed */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastSimpleResultDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/autostart': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get Xbox broadcast auto-start setting */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current setting */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastAutoStartDTO'];
          };
        };
      };
    };
    put?: never;
    /** Set Xbox broadcast auto-start */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['BroadcastAutoStartSetRequestDTO'];
        };
      };
      responses: {
        /** @description Setting updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastAutoStartDTO'];
          };
        };
        /** @description invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/credentials': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the host-wide MCXboxBroadcast account status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Configured account identity and password presence; the password is never returned */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastCredentialsStatusDTO'];
          };
        };
        /** @description credential_status_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    /** Update the host-wide MCXboxBroadcast Microsoft account credentials */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['BroadcastCredentialsDTO'];
        };
      };
      responses: {
        /** @description result: credentials_updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': {
              result: string;
            } & {
              [key: string]: unknown;
            };
          };
        };
        /** @description missing_body / invalid_json / missing_fields */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description update_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/download-jar': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Download the MCXboxBroadcast JAR as a cancellable managed operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Download started/finished */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastJarDownloadResultDTO'];
          };
        };
        /** @description Download accepted; operationId is populated */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastJarDownloadResultDTO'];
          };
        };
        /** @description Conflict */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/jar-status': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get MCXboxBroadcast JAR install status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastJarStatusDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/restart': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Restart Xbox broadcast as a cancellable managed operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description result: broadcast_restart_requested */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastSimpleResultDTO'];
          };
        };
        /** @description Restart accepted; operationId is populated */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastSimpleResultDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/start': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Start Xbox broadcast as a cancellable managed operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description result: broadcast_start_requested */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastSimpleResultDTO'];
          };
        };
        /** @description Start accepted; operationId is populated */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastSimpleResultDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/status': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get Xbox/Bedrock broadcast running status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastStatusDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/broadcast/stop': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Stop Xbox broadcast as a managed operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description result: broadcast_stop_requested */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastSimpleResultDTO'];
          };
        };
        /** @description Stop accepted; operationId is populated and is not cancellable after graceful termination begins */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['BroadcastSimpleResultDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/capabilities': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Report agent capabilities for this host and this token */
    get: {
      parameters: {
        query?: {
          javaFlavor?: string;
          /** @description Optional executable to probe without changing the saved Java preference. */
          javaRuntimePath?: string;
          loaderVersion?: string;
          /** @description Optional selected Minecraft version or version-entry id. */
          minecraftVersion?: string;
          /** @description Optional create-time server edition to evaluate instead of the active server. */
          serverType?: 'java' | 'bedrock';
        };
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description CapabilitiesDTO. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['CapabilitiesDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/catalog/projects/{projectId}': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Fetch full Modrinth project detail for the catalog browser */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path: {
          projectId: string;
        };
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description full project detail */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['CatalogProjectDetailDTO'];
          };
        };
        /** @description provider_unavailable */
        502: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/catalog/projects/{projectId}/versions': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Fetch every Modrinth version for a catalog project */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path: {
          projectId: string;
        };
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description all project versions; compatibility filtering is client-side */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['CatalogVersionsResponseDTO'];
          };
        };
        /** @description provider_unavailable */
        502: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/catalog/search': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Search the Modrinth add-on catalog for the active server, or an explicit flavor with no active server needed (query params q, offset, javaFlavor, minecraftVersion) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description search results (always 200; supportsAddons=false + note conveys unsupported states) */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['CatalogSearchResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/command': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Send a console command to the active server */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['CommandRequest'];
        };
      };
      responses: {
        /** @description Command sent */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['CommandResult'];
          };
        };
        /** @description missing_body / missing_command / invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description capability_unavailable when a Bedrock runtime cannot supply the command */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/components': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Installed system components (Paper/Geyser/Floodgate/flavor jar) and update status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Installed system components (Paper/Geyser/Floodgate/flavor jar) and update status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ComponentsStatusDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/components/client-export': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Package selected components/add-ons for client-side install (query param selected, comma-separated ids) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description export payload */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ClientExportResponseDTO'];
          };
        };
        /** @description no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/components/install': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Install an add-on from the Modrinth catalog into the active server */
    post: operations['installComponent'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/components/remove': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Remove an installed Modrinth-tracked add-on */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['AddonRemoveRequestDTO'];
        };
      };
      responses: {
        /** @description Add-on removed */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['AddonRemoveResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_jar_stem */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description not_supported */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/components/update': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Update a system component (paper/geyser/floodgate) or a Modrinth-tracked add-on */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ComponentUpdateRequestDTO'];
        };
      };
      responses: {
        /** @description Synchronous shapes only (toggle/link/source-set/source-remove): updateAll/jarStem-update shapes instead return 202 (see below) once request shape and the pack-managed guard pass. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json':
              | components['schemas']['AddonUpdateResultDTO']
              | components['schemas']['ComponentUpdateResultDTO'];
          };
        };
        /** @description updateAll/jarStem-update shapes only: update admitted and started. result/count land on the operation's terminal AddonUpdateResultDTO. */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['AddonUpdateResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_component_or_jar_stem / unknown_component */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description not_found (jarStem path) */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description not_supported (jarStem path) / pack_managed (ErrorDTO.details carries packName/packVersion; applies to every shape below except the legacy component=paper|geyser|floodgate path, which never touches an add-on) */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/components/version': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Change the active server's JAR version/build */
    post: operations['changeVersion'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/config/curseforge': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get whether this agent has a CurseForge API key */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description CurseForge API key status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['CurseForgeApiKeyStatusDTO'];
          };
        };
        /** @description credential_status_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    /** Save or clear this agent's CurseForge API key */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['CurseForgeApiKeyUpdateDTO'];
        };
      };
      responses: {
        /** @description CurseForge API key status after the update */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['CurseForgeApiKeyStatusDTO'];
          };
        };
        /** @description invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description credential_store_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/config/geyser': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the active server's Geyser config */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current config */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['GeyserConfigResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Update the active server's Geyser config */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['GeyserConfigUpdateRequestDTO'];
        };
      };
      responses: {
        /** @description Config updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['GeyserConfigUpdateResultDTO'];
          };
        };
        /** @description invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_server / not_installed */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description write_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/config/host-setup': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get whether one-time setup is complete for this agent host */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current host setup state */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HostSetupStateDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/config/host-setup/complete': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Mark one-time setup complete for this agent host */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Host setup marked complete */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HostSetupStateDTO'];
          };
        };
        /** @description set_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/config/java-runtime': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the global Java executable path override */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current path */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['JavaConfigResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Set the global Java executable path override */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['JavaConfigSetRequestDTO'];
        };
      };
      responses: {
        /** @description Path updated, echoes fresh config */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['JavaConfigResponseDTO'];
          };
        };
        /** @description missing_body / invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description set_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/config/ram': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the active server's RAM allocation */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current allocation */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['RAMConfigResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Update the active server's RAM allocation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['RAMConfigUpdateRequestDTO'];
        };
      };
      responses: {
        /** @description Allocation updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['RAMConfigUpdateResultDTO'];
          };
        };
        /** @description invalid_json / no_changes */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/config/servers-root': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the folder where this agent stores servers */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current servers root */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServersRootResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Set the folder where this agent stores servers */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ServersRootSetRequestDTO'];
        };
      };
      responses: {
        /** @description Servers root updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServersRootResponseDTO'];
          };
        };
        /** @description invalid_servers_root */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description set_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/connectivity': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Reachability summary: join address, method, playit/broadcast status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Reachability summary: join address, method, playit/broadcast status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ConnectivityResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/console/tail': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Last N console lines (query param n, default 200, clamped 1-2000) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description console lines */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ConsoleLineDTO'][];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/duckdns': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get the configured DuckDNS hostname */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['DuckDNSStatusResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Update the DuckDNS hostname */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['DuckDNSUpdateRequestDTO'];
        };
      };
      responses: {
        /** @description Hostname updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['DuckDNSUpdateResultDTO'];
          };
        };
        /** @description invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description success: false */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/files': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Browse the active server's directory (admin-only; query param path). 409 reuses the same shape with note=no_active_server. */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description directory listing */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerFilesResponseDTO'];
          };
        };
        /** @description forbidden (non-admin token) */
        403: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/files/read': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Read a previewable file's contents (admin-only; query param path, required) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description file contents */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerFileReadResponseDTO'];
          };
        };
        /** @description missing_path */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description forbidden (non-admin token) */
        403: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description file_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description directory_not_file / not_previewable / no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description read_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/guides/onboarding': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Read the structured first-launch guide */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description OnboardingGuideDTO. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['OnboardingGuideDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/guides/router-catalog': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List router guides and troubleshooting topics */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description RouterGuideCatalogDTO. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['RouterGuideCatalogDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/guides/router/{guideId}': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Compose and resolve one router guide */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path: {
          guideId: string;
        };
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Composed guide with runtime token replacements. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ResolvedRouterGuideDTO'];
          };
        };
        /** @description Unknown router guide. */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description No server is selected for runtime token resolution. */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/guides/router/search': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Search and match router guides */
    get: {
      parameters: {
        query: {
          /** @description Provider, router, model, or troubleshooting query. */
          q: string;
        };
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Ranked router-guide matches and fallback resolution. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['RouterGuideSearchDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/guides/router/troubleshooting/analyze': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Analyze router troubleshooting symptoms */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['RouterTroubleshootingAnalyzeRequestDTO'];
        };
      };
      responses: {
        /** @description Prioritized causes and recommended actions. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['RouterTroubleshootingAnalyzeResponseDTO'];
          };
        };
        /** @description Invalid JSON or unknown symptom id. */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/health': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Diagnostic health cards for the active server */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Diagnostic health cards for the active server */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HealthResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/health/problems': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Startup problems detected for the active server (missing deps, incompatible versions, ...) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Startup problems detected for the active server (missing deps, incompatible versions, ...) */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HealthProblemsResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/health/repair': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Trigger a repair action for a diagnosed startup problem */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['HealthRepairRequestDTO'];
        };
      };
      responses: {
        /** @description action=disable|delete only (synchronous rename/removal). action=update|install instead returns 202 (see below). */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HealthRepairResultDTO'];
          };
        };
        /** @description action=update|install only: repair admitted and started through the same verified add-on mutation path ordinary install/update use (P8.23). success/message/updated land on the operation's terminal HealthRepairResultDTO. */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HealthRepairResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_problem_id / invalid_action / action_unavailable */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description problem_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_running / no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/help/{helpId}': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Resolve an educational content topic */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path: {
          helpId: string;
        };
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description HelpTopicDTO. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HelpTopicDTO'];
          };
        };
        /** @description Unknown helpId -- a normal, expected case, not a server fault. */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/help/catalog': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List available educational content topics */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description HelpCatalogDTO. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HelpCatalogDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/host/reset': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /**
     * Reset this host's MSC state
     * @description Host-scoped, administrator-only reset. The request has no host selector: it always acts on the host serving it. The operation revokes all existing credentials, rotates the host identity, clears host setup/configuration, and either preserves the managed server tree (configuration) or removes it (everything). The route never installs or uninstalls an operating-system service; a local desktop owns that separate action.
     */
    post: operations['resetHost'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/java-runtimes': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Java runtimes detected on this host */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Java runtimes detected on this host */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['JavaRuntimesResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/java-runtimes/install': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Install a Java runtime the agent manages itself */
    post: operations['installJavaRuntime'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/me': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** The calling token's own role and permissions */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description The calling token's own role and permissions */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['MeResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/modpacks/{operationId}/manual-file': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Complete one pending author-blocked CurseForge file for a running modpack-import operation (D-027) */
    post: operations['completeModpackManualFile'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/modpacks/import': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Import a staged modpack archive into the active server, or explicitly replace an already pack-managed server's pack */
    post: operations['importModpack'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/modpacks/inspect': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Inspect a staged modpack archive (.mrpack or CurseForge) without mutating any server */
    post: operations['inspectModpack'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/operations': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Create a long-running operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['OperationCreateRequestDTO'];
        };
      };
      responses: {
        /** @description Accepted for asynchronous processing. Always state: queued at creation. */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['OperationDTO'];
          };
        };
        /** @description Unrecognized type. */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/operations/{id}': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Read an operation's current state */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path: {
          id: string;
        };
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current OperationDTO. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['OperationDTO'];
          };
        };
        /** @description Unknown id -- this phase, includes any operation forgotten across a restart (no journal yet). */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/operations/{id}/cancel': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Request cancellation of an operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path: {
          id: string;
        };
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Cancellation atomically accepted with a captured non-terminal OperationDTO, normally running with statusLine: Cancelling. Poll or stream until terminal. */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['OperationDTO'];
          };
        };
        /** @description Unknown id. */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description The worker reached a terminal state before cancellation admission. */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/performance': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Latest performance snapshot (TPS, players, CPU, RAM, world size) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Latest performance snapshot (TPS, players, CPU, RAM, world size) */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PerformanceSnapshotDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List currently-online players */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Online players */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayersResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/{profileId}/skin': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** A player's skin image (base64), by profile id. Handled outside the main route switch via a path.hasPrefix/hasSuffix match, not a `case` in it -- the one MSC 1 route with a path parameter. */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description skin image */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerSkinResponseDTO'];
          };
        };
        /** @description missing_profile_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description lookup failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/delete': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Delete a player's data */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['PlayerDeleteRequestDTO'];
        };
      };
      responses: {
        /** @description Player data deleted */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_profile_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / not_bedrock */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/duplicate': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Duplicate a player's data */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['PlayerDeleteRequestDTO'];
        };
      };
      responses: {
        /** @description Player data duplicated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_profile_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / not_bedrock */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/hidden': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Hide or unhide a player profile */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['HiddenProfileMutationRequestDTO'];
        };
      };
      responses: {
        /** @description Visibility updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['HiddenProfileMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_profile_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/identify': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Assign a gamertag to an unresolved Bedrock profile */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['PlayerIdentifyRequestDTO'];
        };
      };
      responses: {
        /** @description Gamertag assigned */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerIdentifyResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_profile_id / missing_gamertag */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / not_bedrock -- this route only applies to a Bedrock profileId (xuid_ prefix); a bare Java UUID profileId is rejected here since Java usernames already resolve from usercache.json */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/migrate': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Migrate player data to a custom UUID */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['PlayerMigrateRequestDTO'];
        };
      };
      responses: {
        /** @description Player data migrated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_profile_id / invalid_uuid */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / not_bedrock */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/migrate-offline': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Migrate player data to its offline UUID */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['PlayerDeleteRequestDTO'];
        };
      };
      responses: {
        /** @description Player data migrated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_profile_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / not_bedrock / username_unknown */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/profiles': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List all-time player profiles with stats */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Player profiles */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerProfilesResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/players/skin-override': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Set or clear a manual skin lookup override for a player */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['PlayerSkinOverrideRequestDTO'];
        };
      };
      responses: {
        /** @description Override set/cleared */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayerSkinOverrideResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_profile_id / invalid_profile_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description profile_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/playit': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get Playit tunnel status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current status */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayitStatusResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/playit/reset': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /**
     * Clear host-local Playit credentials and derived state
     * @description Stops every MSC-managed Playit helper before clearing the host-scoped agent key, saved agent ID, public addresses, and setup prompt state. The operation is idempotent: an already-clear host returns success. It never deletes Playit cloud agents or tunnels.
     */
    post: operations['resetPlayit'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/playit/setup': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /**
     * Start native Playit account and tunnel setup
     * @description Accepts Playit email and password only through the authenticated MSC agent API. The agent signs in, claims or reuses the host's Playit agent, creates or reuses applicable tunnels, and reports progress through the shared operation routes. Credentials and temporary Playit session details are memory-only; the resulting agent key is stored in the host secret store and is never returned.
     */
    post: operations['setupPlayit'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/playit/start': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Start the Playit tunnel as a cancellable managed operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description started / already_running */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayitActionResultDTO'];
          };
        };
        /** @description Tunnel start accepted; operationId is populated */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayitActionResultDTO'];
          };
        };
        /** @description not_enabled / no_secret_key / no_server / helper_unavailable */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/playit/stop': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Stop the Playit tunnel as a managed operation */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description stopped / not_running */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayitActionResultDTO'];
          };
        };
        /** @description Tunnel stop accepted; operationId is populated and is not cancellable after graceful termination begins */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['PlayitActionResultDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/resourcepacks': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List resource packs */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current packs */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ResourcePacksResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/resourcepacks/activate': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Activate (or clear) the local Java resource pack */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ResourcePackActivateRequestDTO'];
        };
      };
      responses: {
        /** @description Pack activated/cleared */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ResourcePackMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / java_only */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/resourcepacks/remove': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Remove a resource pack from disk */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ResourcePackRemoveRequestDTO'];
        };
      };
      responses: {
        /** @description Pack removed */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ResourcePackMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description pack_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/resourcepacks/seturl': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Set a custom resource-pack URL directly in server.properties */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ResourcePackSetURLRequestDTO'];
        };
      };
      responses: {
        /** @description URL set */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ResourcePackMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / url_required */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / java_only */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/resourcepacks/toggle': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Enable/disable a Geyser resource pack */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ResourcePackToggleRequestDTO'];
        };
      };
      responses: {
        /** @description Pack toggled */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ResourcePackMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description pack_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List all registered servers */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description List all registered servers */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerDTO'][];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers/create': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Create a new server */
    post: operations['createServer'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers/delete': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Delete a server */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ServerDeleteRequestDTO'];
        };
      };
      responses: {
        /** @description Server deleted */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerDeleteResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_server_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_running */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description delete_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers/directory': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /**
     * Update a server's configured directory
     * @description Updates the configured path only; it does not move or create files on disk.
     */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ServerDirectoryRequestDTO'];
        };
      };
      responses: {
        /** @description Server directory updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerDirectoryResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_server_id / directory_required */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers/eula': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Accept the Minecraft EULA for a server */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ServerEULARequestDTO'];
        };
      };
      responses: {
        /** @description EULA accepted */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerEULAResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_server_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description eula_write_failed */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers/import': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Scan, import, or rescan existing servers */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ServerImportRequestDTO'];
        };
      };
      responses: {
        /** @description action=scan returns ServerImportScanResponseDTO synchronously. */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerImportScanResponseDTO'];
          };
        };
        /** @description importExisting/importTransfer/rescan accepted as a durable background operation. Poll operationId for the final result. */
        202: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerImportResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / invalid_action / missing_source_path / invalid_path / display_name_required / backup_path_required (typed for every cause except missing_body/invalid_json (still Error)); sourcePath is not required for action=rescan */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description source_not_found (typed for every cause except missing_body/invalid_json (still Error)) */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_running (typed for every cause except missing_body/invalid_json (still Error)) */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error (typed for every cause except missing_body/invalid_json (still Error)) */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers/rename': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Rename a server */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['ServerRenameRequestDTO'];
        };
      };
      responses: {
        /** @description Server renamed */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerRenameResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_server_id / name_required */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_running */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/servers/size': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Measure a registered server's directory */
    get: {
      parameters: {
        query: {
          serverId: string;
        };
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Directory size */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ServerDirectorySizeResponseDTO'];
          };
        };
        /** @description missing_server_id */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/session-log': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Join/leave event history for the active server */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Join/leave event history for the active server */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['SessionLogResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/session-log/clear': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Clear join/leave event history for the active server */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Session log cleared */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['SessionLogResponseDTO'];
          };
        };
        /** @description no_active_server */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/settings': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Typed server.properties schema for the active server, as sections of fields */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Typed server.properties schema for the active server, as sections of fields */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['SettingsResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Apply a sparse set of settings changes (key -> string value), validated and clamped per field */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['SettingsUpdateRequestDTO'];
        };
      };
      responses: {
        /** @description applied */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['SettingsUpdateResultDTO'];
          };
        };
        /** @description missing_body / invalid_json (no_valid_changes uses SettingsUpdateResultDTO instead, see notes) */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / not_supported */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/staged-downloads/{id}': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Download bytes from a previously prepared staged export */
    get: operations['downloadStagedBytes'];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/staged-uploads': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Begin a bounded staged upload */
    post: operations['beginStagedUpload'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/staged-uploads/{id}': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    /** Upload bytes into a previously begun staging slot */
    put: operations['uploadStagedBytes'];
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/start': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Start the active server */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description result: start_requested */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['SimpleResult'];
          };
        };
        /** @description capability_unavailable when the selected Bedrock runtime cannot start */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/status': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Current run status (active server, pid, running state) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current run status (active server, pid, running state) */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['RemoteAPIStatus'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/stop': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Stop the active server */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description result: stop_requested */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['SimpleResult'];
          };
        };
        /** @description capability_unavailable when the selected Bedrock runtime cannot stop */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/templates': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List available Paper/plugin templates */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Current templates */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['TemplatesResponseDTO'];
          };
        };
      };
    };
    put?: never;
    /** Export the active server as a template, or create a server from one */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['TemplateMutationRequestDTO'];
        };
      };
      responses: {
        /** @description Mutation applied */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['TemplateMutationResultDTO'];
          };
        };
        /** @description invalid_action / name_required / template_required / missing_server_id / missing_source_path / invalid_path (typed for every cause except missing_body/invalid_json (still Error)) */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_not_found / template_not_found (typed for every cause except missing_body/invalid_json (still Error)) */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description server_running / unsupported_template (typed for every cause except missing_body/invalid_json (still Error)) */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error (typed for every cause except missing_body/invalid_json (still Error)) */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/users': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List named-access users */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description User list */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['UserListResponseDTO'];
          };
        };
        /** @description forbidden */
        403: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    put?: never;
    /** Create a named-access user */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['UserCreateRequestDTO'];
        };
      };
      responses: {
        /** @description User created (token returned once) */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['UserCreateResultDTO'];
          };
        };
        /** @description invalid_json / label_empty */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description invalid_role / invalid_permissions */
        422: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/users/revoke': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Revoke a named-access user */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['UserRevokeRequestDTO'];
        };
      };
      responses: {
        /** @description User revoked */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['UserRevokeResultDTO'];
          };
        };
        /** @description invalid_json */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/users/update': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Update a named-access user's label/role/permissions/expiry */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['UserUpdateRequestDTO'];
        };
      };
      responses: {
        /** @description User updated */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['UserUpdateResultDTO'];
          };
        };
        /** @description invalid_json / label_empty */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description invalid_role / invalid_permissions */
        422: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/versions': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Available server JAR versions for the active server's flavor */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description Available server JAR versions for the active server's flavor */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['VersionsResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/versions/create': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Available versions for the create-server flow (query params serverType, javaFlavor) */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description versions */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['VersionsResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/watchdog/disable': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Disable the watchdog */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description success: "true" */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WatchdogActionResultDTO'];
          };
        };
        /** @description success: "false", error: <message> */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/watchdog/enable': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Enable the watchdog */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description success: "true" */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WatchdogActionResultDTO'];
          };
        };
        /** @description success: "false", error: <message> */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/watchdog/status': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Get watchdog enabled status */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description enabled: "true"|"false" */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WatchdogStatusResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** World slots for the active server, plus which is active */
    get: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        /** @description World slots for the active server, plus which is active */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WorldSlotsResponseDTO'];
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/{slotId}/profile': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** Read one world slot's saved profile and runtime metadata */
    get: operations['getWorldSlotProfile'];
    put?: never;
    /** Save a world slot profile and apply its accepted runtime projection */
    post: operations['updateWorldSlotProfile'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/{slotId}/thumbnail': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** A world slot's thumbnail image, if one was generated */
    get: operations['getWorldSlotThumbnail'];
    put?: never;
    /** Set a world slot's thumbnail from a staged image upload */
    post: operations['setWorldSlotThumbnail'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/activate': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Activate a world slot (starts activation asynchronously) */
    post: operations['activateWorldSlot'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/convert': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Start a Chunker world-format conversion */
    post: operations['convertWorld'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/convert/formats': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    /** List the installed Chunker world-conversion formats */
    get: operations['getWorldConvertFormats'];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/create': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Create a new world slot */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['WorldCreateRequestDTO'];
        };
      };
      responses: {
        /** @description Repair started */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WorldRepairResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_slot_id / name_required */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description slot_not_found / source_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / server_running / bedrock_only */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/delete': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Delete a non-active world slot */
    post: operations['deleteWorldSlot'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/duplicate': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Duplicate a world slot under a fresh id */
    post: operations['duplicateWorldSlot'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/export': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Stage a world slot's archive for download */
    post: operations['exportWorldSlot'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/import': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Import a staged world ZIP as a new slot */
    post: operations['importWorldSlot'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/rename': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Rename a world slot */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['WorldRenameRequestDTO'];
        };
      };
      responses: {
        /** @description Mutation applied */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WorldMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_slot_id / name_required */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description slot_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / server_running */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/rename-active-world': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Directly rename the active/live world's on-disk folders */
    post: operations['renameActiveWorld'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/repair': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Repair a Bedrock world's level.dat */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['WorldRepairRequestDTO'];
        };
      };
      responses: {
        /** @description Repair started -- operation-backed (P12.4e), matching world activation's own shape */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WorldRepairResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_slot_id / name_required */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description slot_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / server_running / bedrock_only / repair_in_progress */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/replace': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Copy a saved slot's content into another existing slot */
    post: {
      parameters: {
        query?: never;
        header?: never;
        path?: never;
        cookie?: never;
      };
      requestBody: {
        content: {
          'application/json': components['schemas']['WorldReplaceRequestDTO'];
        };
      };
      responses: {
        /** @description Mutation applied */
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['WorldMutationResultDTO'];
          };
        };
        /** @description missing_body / invalid_json / missing_slot_id / name_required */
        400: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description slot_not_found / source_not_found */
        404: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description no_active_server / server_running / same_slot */
        409: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
        /** @description internal error */
        500: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            'application/json': components['schemas']['ErrorDTO'];
          };
        };
      };
    };
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/replace-active-world': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Replace the active/live world's on-disk content directly (starts asynchronously) */
    post: operations['replaceActiveWorld'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  '/v1/worlds/update': {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    /** Save the current live world into the active slot */
    post: operations['updateActiveWorldSlot'];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
}
export type webhooks = Record<string, never>;
export interface components {
  schemas: {
    AddonItemDTO: {
      availableVersion?: string;
      bucket: string;
      currentVersion?: string;
      displayName: string;
      iconURL?: string;
      isEnabled: boolean;
      jarStem: string;
      projectId?: string;
    } & {
      [key: string]: unknown;
    };
    AddonRemoveRequestDTO: {
      jarStem: string;
    } & {
      [key: string]: unknown;
    };
    AddonRemoveResultDTO: {
      jarStem: string;
      message: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    AddonsResponseDTO: {
      addons: components['schemas']['AddonItemDTO'][];
      isResolving: boolean;
      /** @description Set when a provider was unreachable during this resolve pass (provider_unavailable) -- addons still reflect last-known persisted state, not a fabricated fresh result. */
      note?: string;
      packManaged: boolean;
      packName?: string;
      serverSupportsAddons: boolean;
    } & {
      [key: string]: unknown;
    };
    AddonUpdateResultDTO: {
      count: number;
      jarStem?: string;
      /** @description Present only for the update-triggering shapes (updateAll, jarStem update) -- these download real bytes and run async. Absent for toggle/link/source-set/source-remove, which stay synchronous (a config or filesystem-rename change, no network fetch). */
      operationId?: string;
      result: string;
    } & {
      [key: string]: unknown;
    };
    AllowlistEntryDTO: {
      ignoresPlayerLimit: boolean;
      name: string;
      xuid?: string;
    } & {
      [key: string]: unknown;
    };
    AllowlistMutationRequestDTO: {
      action: string;
      name: string;
    } & {
      [key: string]: unknown;
    };
    AllowlistMutationResultDTO: {
      entries: components['schemas']['AllowlistEntryDTO'][];
      message: string;
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverType: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    AllowlistResponseDTO: {
      entries: components['schemas']['AllowlistEntryDTO'][];
      /** @description Optional Bedrock runtime state; file reads may remain available while the runtime is unavailable. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverType: string;
    } & {
      [key: string]: unknown;
    };
    BackupConfigResponseDTO: {
      autoBackupEnabled: boolean;
      autoBackupIntervalMinutes: number;
      autoBackupMaxCount: number;
      intervalOptions: number[];
      note?: string;
      /** @description Optional runtime state while backup settings are read. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverName: string;
    } & {
      [key: string]: unknown;
    };
    BackupConfigUpdateRequestDTO: {
      autoBackupEnabled?: boolean;
      autoBackupIntervalMinutes?: number;
      autoBackupMaxCount?: number;
    } & {
      [key: string]: unknown;
    };
    BackupConfigUpdateResultDTO: {
      config?: components['schemas']['BackupConfigResponseDTO'];
      message: string;
      /** @description Optional runtime state after backup settings are applied. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    BackupDeleteRequestDTO: {
      backupId: string;
    } & {
      [key: string]: unknown;
    };
    BackupItemDTO: {
      displayName: string;
      fileSize?: number;
      id: string;
      isAutomatic: boolean;
      modificationDate?: string;
      slotId?: string;
      slotName?: string;
      triggerReason: string;
    } & {
      [key: string]: unknown;
    };
    BackupNowResultDTO: {
      /** @description Operation id for progress polling (GET /v1/operations/{id}) or /v1/operations/{id}/stream and cancellation; optional so older clients can ignore it, matching SimpleResult's P4 precedent. */
      operationId?: string;
      result: string;
      /** @description Optional runtime state for the backup operation. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
    } & {
      [key: string]: unknown;
    };
    BackupRestoreRequestDTO: {
      backupId: string;
    } & {
      [key: string]: unknown;
    };
    BackupRestoreResultDTO: {
      /** @description Operation id for progress polling (GET /v1/operations/{id}) or /v1/operations/{id}/stream and cancellation; optional so older clients can ignore it, matching SimpleResult's P4 precedent. */
      operationId?: string;
      result: string;
      /** @description Optional runtime state for the restore operation. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
    } & {
      [key: string]: unknown;
    };
    BackupsResponseDTO: {
      backups: components['schemas']['BackupItemDTO'][];
      /** @description Optional runtime state for the active server's backup source. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
    } & {
      [key: string]: unknown;
    };
    /** @description Phase 10 runtime disclosure shared by existing v1 DTOs. It describes the current Bedrock backend state, not the separate published compatibility matrix. */
    BedrockRuntimeStateDTO: {
      /** @enum {string|null} */
      backend?: 'native' | 'vz-sidecar' | null;
      /** @description Optional educational topic for the runtime reason. */
      helpId?: string | null;
      /** @enum {string|null} */
      hostOs?: 'macos' | 'linux' | 'windows' | null;
      message?: string | null;
      /** @enum {string|null} */
      reasonCode?:
        | 'no_test_hardware'
        | 'unsupported_host'
        | 'missing_sidecar'
        | 'missing_bds'
        | 'not_provisioned'
        | 'verification_failed'
        | 'port_unavailable'
        | 'sidecar_unavailable'
        | 'not_detected'
        | null;
      /** @enum {string} */
      state: 'available' | 'provisioning_required' | 'unavailable';
    } & {
      [key: string]: unknown;
    };
    BroadcastAuthPromptDTO: {
      code?: string;
      isPresent: boolean;
      linkURL?: string;
    } & {
      [key: string]: unknown;
    };
    BroadcastAutoStartDTO: {
      enabled: boolean;
    } & {
      [key: string]: unknown;
    };
    BroadcastAutoStartSetRequestDTO: {
      enabled: boolean;
    } & {
      [key: string]: unknown;
    };
    BroadcastCredentialsDTO: {
      email: string;
      gamertag: string;
      password: string;
    } & {
      [key: string]: unknown;
    };
    BroadcastCredentialsStatusDTO: {
      /** @description Configured Microsoft account email, when present. */
      email?: string;
      /** @description Configured Xbox gamertag, when present. */
      gamertag?: string;
      /** @description Whether a password is stored in the host secret store. */
      hasPassword: boolean;
    } & {
      [key: string]: unknown;
    };
    BroadcastJarDownloadResultDTO: {
      filename?: string;
      message: string;
      /** @description Present when the staged download continues after this response. */
      operationId?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    BroadcastJarStatusDTO: {
      downloading: boolean;
      filename?: string;
      installed: boolean;
    } & {
      [key: string]: unknown;
    };
    BroadcastSimpleResultDTO: {
      /** @description Present when helper lifecycle work continues after this response. */
      operationId?: string;
      result: string;
    } & {
      [key: string]: unknown;
    };
    BroadcastStatusDTO: {
      bedrockBroadcastRunning: boolean;
      /** @description The Xbox gamertag reported by MCXboxBroadcast after authentication, when available. */
      gamertag?: string;
      xboxBroadcastRunning: boolean;
    } & {
      [key: string]: unknown;
    };
    BrowserSessionExchangeRequestDTO: {
      pairingCode: string;
    } & {
      [key: string]: unknown;
    };
    /** @description P2.6 SS3 -- GET /v1/capabilities response; P12.28 adds optional version-aware worldSettings. */
    CapabilitiesDTO: {
      agentVersion: string;
      apiMajor: number;
      apiMinor: number;
      /** @description Placeholder this phase (P2.6 SS3) -- real presence detection is Phase 3 substrate work. */
      helpers: {
        duckdns: boolean;
        geyser: boolean;
        playit: boolean;
        /** @description Optional installed-state probe for the Tailscale helper. Absent on older agents. */
        tailscale?: boolean;
      } & {
        [key: string]: unknown;
      };
      /** @enum {string} */
      hostOs: 'macos' | 'linux' | 'windows';
      /** @description The calling token's granted categories, per D-019's nine-bucket vocabulary (Proposed). */
      permissions: (
        | 'serverControl'
        | 'players'
        | 'settings'
        | 'addons'
        | 'worlds'
        | 'broadcast'
        | 'networking'
        | 'fleet'
        | 'admin'
      )[];
      /** @description Java flags and Bedrock runtime state are host capabilities. Bedrock uses the separate D-022 compatibility matrix for published evidence. */
      serverTypes: {
        bedrock: {
          /** @enum {string|null} */
          backend: 'native' | 'vz-sidecar' | null;
          /** @description Authoritative current runtime state; supported/backend remain for older clients. */
          runtime?: components['schemas']['BedrockRuntimeStateDTO'];
          supported: boolean;
        } & {
          [key: string]: unknown;
        };
        fabric: boolean;
        forge: boolean;
        neoforge: boolean;
        paper: boolean;
        vanilla: boolean;
      } & {
        [key: string]: unknown;
      };
      /** @description Optional active or create-time context for native world settings. Omitted when no server context was selected. */
      worldSettings?: components['schemas']['WorldSettingsCapabilitiesDTO'];
    } & {
      [key: string]: unknown;
    };
    CatalogGalleryImageDTO: {
      description?: string | null;
      featured: boolean;
      title?: string | null;
      url: string;
    } & {
      [key: string]: unknown;
    };
    CatalogInstallRequestDTO: {
      projectId?: string;
      slug?: string;
      /** @description Install from a staged local JAR (purpose addon-local-file) instead of the catalog. Exactly one of {projectId+slug+title} or stagedUploadId must be given -- not both, not neither. */
      stagedUploadId?: string;
      title?: string;
      /** @description Install this exact Modrinth version instead of resolving the latest compatible one server-side. When present, projectId/slug/title are still used for the install-result message and staging metadata; the version is fetched directly by id (GET /v2/version/{id}) rather than searched for. */
      versionId?: string;
    } & {
      [key: string]: unknown;
    };
    CatalogInstallResultDTO: {
      /** @description jarStems of required dependencies installed alongside the requested add-on (P8.15's bounded dependency installer). Empty when the add-on had none. */
      installedDependencies?: string[];
      message: string;
      /** @description Operation id for progress polling (GET /v1/operations/{id}) or /v1/operations/{id}/stream and cancellation; optional so older clients can ignore it. */
      operationId?: string;
      projectId: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    CatalogItemDTO: {
      author: string;
      description: string;
      downloads: number;
      iconURL?: string;
      isClientOnly: boolean;
      projectId: string;
      projectType: string;
      slug: string;
      title: string;
    } & {
      [key: string]: unknown;
    };
    CatalogProjectDetailDTO: {
      /** @description Full Markdown/HTML project body (Modrinth's 'body' field). Client is responsible for safely rendering it -- this is untrusted third-party content. */
      body: string;
      description: string;
      discordURL?: string | null;
      downloads: number;
      followers: number;
      gallery: components['schemas']['CatalogGalleryImageDTO'][];
      iconURL?: string | null;
      issuesURL?: string | null;
      projectId: string;
      /** @description One of required/optional/unsupported, Modrinth's own vocabulary -- unchanged, not remapped to a boolean. */
      serverSide: string;
      slug: string;
      sourceURL?: string | null;
      title: string;
      wikiURL?: string | null;
    } & {
      [key: string]: unknown;
    };
    CatalogSearchResponseDTO: {
      addonKind?: string;
      gameVersion?: string;
      loaderName?: string;
      note?: string;
      results?: components['schemas']['CatalogItemDTO'][];
      supportsAddons: boolean;
    } & {
      [key: string]: unknown;
    };
    CatalogVersionDependencyDTO: {
      /** @description required/optional/incompatible/embedded */
      dependencyType: string;
      projectId?: string | null;
      versionId?: string | null;
    } & {
      [key: string]: unknown;
    };
    CatalogVersionDTO: {
      datePublished?: string | null;
      dependencies: components['schemas']['CatalogVersionDependencyDTO'][];
      files: components['schemas']['CatalogVersionFileDTO'][];
      gameVersions: string[];
      id: string;
      loaders: string[];
      name: string;
      projectId: string;
      versionNumber: string;
      /** @description release/beta/alpha */
      versionType: string;
    } & {
      [key: string]: unknown;
    };
    CatalogVersionFileDTO: {
      filename: string;
      primary: boolean;
      size?: number | null;
      url: string;
    } & {
      [key: string]: unknown;
    };
    CatalogVersionsResponseDTO: {
      versions: components['schemas']['CatalogVersionDTO'][];
    } & {
      [key: string]: unknown;
    };
    ClientExportItemDTO: {
      clientStatus: string;
      displayName: string;
      fileName: string;
      iconURL?: string;
      id: string;
      projectURL?: string;
      selectedByDefault: boolean;
      statusSource: string;
    } & {
      [key: string]: unknown;
    };
    ClientExportResponseDTO: {
      exportKind: string;
      isPaperLike: boolean;
      items: components['schemas']['ClientExportItemDTO'][];
      note?: string;
      selectedCount: number;
      serverName?: string;
      serverType: string;
      shareText?: string;
      /** @description Set only for exportKind=zip. Redeem via GET /v1/staged-downloads/{id} (P6.8's staged-download primitive), the same mechanism POST /v1/worlds/export already uses -- not inline base64, which would roughly double a large modded pack's response size. */
      stagedDownloadId?: string;
      zipFileName?: string;
    } & {
      [key: string]: unknown;
    };
    CommandRequest: {
      command: string;
      /** @description Acknowledgement token returned in a confirmation_required error before sending a Creative-changing command. */
      confirmation?: string;
    } & {
      [key: string]: unknown;
    };
    CommandResult: {
      activeServerId?: string;
      command: string;
      result: string;
      /** @description Optional runtime state for a Bedrock command result. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
    } & {
      [key: string]: unknown;
    };
    ComponentsStatusDTO: {
      components: components['schemas']['ComponentStatusDTO'][];
      restartRequiredToApply: boolean;
    } & {
      [key: string]: unknown;
    };
    ComponentStatusDTO: {
      installedBuild?: number;
      installedLabel?: string;
      installedVersion?: string;
      isUpToDate: boolean;
      latestBuild?: number;
      latestVersion?: string;
      name: string;
      /** @description Set for a component this build honestly cannot check yet (e.g. Geyser/Floodgate update checks stay Phase 9) instead of a fabricated isUpToDate/updatable value. */
      note?: string;
      updatable: boolean;
    } & {
      [key: string]: unknown;
    };
    ComponentUpdateRequestDTO: {
      component?: string;
      /** @description With jarStem and no updateAll/component: enable/disable that add-on (togglePlugin/toggleMod). */
      enabled?: boolean;
      jarStem?: string;
      /** @description With jarStem: manually link this add-on to a Modrinth project id (manuallyLinkAddon). Sets AddonLinkProvenance.nameGuess. */
      linkProjectId?: string;
      /** @description With jarStem: remove a previously-set plugin source (removePluginSource). */
      removeSource?: boolean;
      /** @description With jarStem: set/replace this add-on's plugin source (GitHub/Hangar/direct URL, classified the same way PluginSourceDetector.detect does). setPluginSource. */
      sourceUrl?: string;
      updateAll?: boolean;
    } & {
      [key: string]: unknown;
    };
    ComponentUpdateResultDTO: {
      message: string;
      newBuild?: number;
      newVersion?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    ConnectivityBroadcastDTO: {
      bedrockRunning: boolean;
      xboxRunning: boolean;
    } & {
      [key: string]: unknown;
    };
    ConnectivityPlayitDTO: {
      address?: string;
      enabled: boolean;
      running: boolean;
    } & {
      [key: string]: unknown;
    };
    ConnectivityPortDiagnosticDTO: {
      detail?: string;
      helpId?: string | null;
      /** @enum {string} */
      outcome: 'open' | 'closed' | 'unreachable' | 'unavailable' | 'not_applicable';
    } & {
      [key: string]: unknown;
    };
    ConnectivityPortDiagnosticsDTO: {
      local: components['schemas']['ConnectivityPortDiagnosticDTO'];
      public: components['schemas']['ConnectivityPortDiagnosticDTO'];
    } & {
      [key: string]: unknown;
    };
    ConnectivityResponseDTO: {
      broadcast?: components['schemas']['ConnectivityBroadcastDTO'];
      detail?: string;
      externallyReachable?: boolean;
      headline: string;
      /** @description Alongside method, e.g. connectivity.method.playit (helpid-contract.md SS4). */
      helpId?: string | null;
      joinAddress?: string;
      /** @enum {string} */
      joinAddressSource?: 'playit' | 'duckdns' | 'public_ip' | 'unavailable';
      localListening?: boolean;
      method: string;
      motd?: string;
      note?: string;
      playersMax?: number;
      playersOnline?: number;
      playit?: components['schemas']['ConnectivityPlayitDTO'];
      portDiagnostics?: components['schemas']['ConnectivityPortDiagnosticsDTO'];
      serverName: string;
      serverRunning: boolean;
      serverType: string;
      severity: string;
      status: string;
    } & {
      [key: string]: unknown;
    };
    ConsoleLineDTO: {
      level?: string;
      source: string;
      text: string;
      ts: string;
    } & {
      [key: string]: unknown;
    };
    CsrfTokenResponseDTO: {
      /** @description Opaque token echoed in X-MSC-CSRF for cookie-authenticated mutations. */
      csrfToken: string;
      expiresAt: string;
    } & {
      [key: string]: unknown;
    };
    CurseForgeApiKeyStatusDTO: {
      /** @description Whether a CurseForge API key is stored in the host secret store. The key itself is never returned. */
      configured: boolean;
    } & {
      [key: string]: unknown;
    };
    CurseForgeApiKeyUpdateDTO: {
      /** @description The CurseForge API key to store. An empty value clears the stored key. */
      apiKey: string;
    } & {
      [key: string]: unknown;
    };
    DesktopCredentialResultDTO: {
      agentHostId: string;
      credentialId: string;
      expiresAt?: string | null;
      /** @description Raw bearer credential, returned only once to the Tauri backend; never expose it to Svelte. */
      token: string;
    } & {
      [key: string]: unknown;
    };
    DesktopPairingExchangeRequestDTO: {
      pairingCode: string;
    } & {
      [key: string]: unknown;
    };
    DuckDNSStatusResponseDTO: {
      hostname?: string;
      isConfigured: boolean;
    } & {
      [key: string]: unknown;
    };
    DuckDNSUpdateRequestDTO: {
      hostname?: string;
    } & {
      [key: string]: unknown;
    };
    DuckDNSUpdateResultDTO: {
      hostname?: string;
      message?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    Error: {
      error: string;
    } & {
      [key: string]: unknown;
    };
    /** @description One error envelope for every non-2xx response across every /v1/ route (P2.4 SS5-6, a D-006-point-3 correction unifying the baseline's split Error/typed-DTO failure pattern). */
    ErrorDTO: {
      /** @description Stable machine-readable snake_case identifier, e.g. not_found, conflict, invalid_body. */
      code: string;
      /** @description Optional free-form structured context. Bedrock capability_unavailable responses use the listed fields when present. */
      details?:
        | ({
            backend?: string | null;
            capability?: string;
            /** @description Structured confirmation required before the requested safety-sensitive change may be applied. */
            confirmation?: {
              acknowledgement: string;
              /** @enum {string} */
              kind:
                | 'bedrock_achievements'
                | 'java_creative'
                | 'java_commands'
                | 'server_force_gamemode';
              message: string;
              /** @enum {string} */
              scope: 'world' | 'server';
              title: string;
            } & {
              [key: string]: unknown;
            };
            hostOs?: string;
            reasonCode?: string;
            serverType?: string;
            state?: string;
          } & {
            [key: string]: unknown;
          })
        | null;
      /** @description Optional pointer into GET /v1/help/{helpId} (P2.2). */
      helpId?: string | null;
      /** @description Human-readable, iOS-visible text. */
      message: string;
    } & {
      [key: string]: unknown;
    };
    GeyserConfigResponseDTO: {
      address?: string;
      configFileExists: boolean;
      isGeyserInstalled: boolean;
      note?: string;
      port?: number;
      serverName: string;
      serverType: string;
    } & {
      [key: string]: unknown;
    };
    GeyserConfigUpdateRequestDTO: {
      address?: string;
      port?: number;
    } & {
      [key: string]: unknown;
    };
    GeyserConfigUpdateResultDTO: {
      address?: string;
      message: string;
      port?: number;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    HealthCardDTO: {
      actionCode?: string;
      actionLabel?: string;
      detail?: string;
      /** @description Alongside detail (helpid-contract.md SS4). */
      helpId?: string | null;
      iconSystemName: string;
      id: string;
      severity: string;
      shortLabel: string;
      title: string;
    } & {
      [key: string]: unknown;
    };
    HealthProblemsResponseDTO: {
      isSoftFail: boolean;
      note?: string;
      problems: components['schemas']['StartupProblemDTO'][];
      serverRunning: boolean;
      serverType: string;
    } & {
      [key: string]: unknown;
    };
    HealthRepairRequestDTO: {
      action: string;
      problemId: string;
    } & {
      [key: string]: unknown;
    };
    HealthRepairResultDTO: {
      message: string;
      /** @description Present only when action is update/install (P8.23 -- these route through the same verified add-on mutation paths as ordinary install/update, so they cost the same real download time). Absent for disable/delete, which stay the existing synchronous rename/removal. */
      operationId?: string;
      success: boolean;
      updated?: components['schemas']['HealthProblemsResponseDTO'];
    } & {
      [key: string]: unknown;
    };
    HealthResponseDTO: {
      cards: components['schemas']['HealthCardDTO'][];
      note?: string;
      overallSeverity: string;
      serverName: string;
      serverRunning: boolean;
      serverType: string;
    } & {
      [key: string]: unknown;
    };
    HelpCatalogDTO: {
      topics: ({
        category: string;
        helpId: string;
        title: string;
      } & {
        [key: string]: unknown;
      })[];
    } & {
      [key: string]: unknown;
    };
    /** @description helpid-contract.md SS3 -- GET /v1/help/{helpId} response. */
    HelpTopicDTO: {
      analogy?: string | null;
      /** @description Raw Markdown. Clients render; they do not author (D-026 point 1). */
      body: string;
      category: string;
      helpId: string;
      relatedIds: string[];
      title: string;
    } & {
      [key: string]: unknown;
    };
    HiddenProfileMutationRequestDTO: {
      hidden: boolean;
      profileId: string;
    } & {
      [key: string]: unknown;
    };
    HiddenProfileMutationResultDTO: {
      isHidden?: boolean;
      message: string;
      profileId?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    /** @description The last reliable response may be returned under the old credential. Poll the operation when the agent remains reachable; otherwise follow the state-specific recovery path. */
    HostResetAcceptedDTO: {
      /**
       * @description Truthful post-admission state: the agent will restart, remains installed but requires fresh pairing, or cannot be reached to report completion.
       * @enum {string}
       */
      agentState: 'restarting' | 'needs_pairing' | 'unavailable';
      /** @description The pre-reset agent host identity, for client-side correlation only. */
      hostId: string;
      message: string;
      /** @enum {string} */
      mode: 'configuration' | 'everything';
      /** @description Journaled reset operation ID. */
      operationId: string;
    } & {
      [key: string]: unknown;
    };
    /** @description A destructive host reset request. The serving agent validates the administrator credential and the fixed confirmation phrase before starting the reset. */
    HostResetRequestDTO: {
      /** @description Must exactly equal RESET AGENT. */
      confirmation: string;
      /**
       * @description configuration clears MSC host state but preserves the managed server tree; everything removes that tree too.
       * @enum {string}
       */
      mode: 'configuration' | 'everything';
    } & {
      [key: string]: unknown;
    };
    HostSetupStateDTO: {
      complete: boolean;
    } & {
      [key: string]: unknown;
    };
    InventoryItemDTO: {
      count: number;
      damage: number;
      displayName: string;
      enchantments: components['schemas']['ItemEnchantmentDTO'][];
      iconName: string;
      itemID: string;
      slot: number;
    } & {
      [key: string]: unknown;
    };
    ItemEnchantmentDTO: {
      displayName: string;
      id: string;
      level: number;
    } & {
      [key: string]: unknown;
    };
    JavaConfigResponseDTO: {
      executablePath?: string;
      extraFlags?: string;
    } & {
      [key: string]: unknown;
    };
    JavaConfigSetRequestDTO: {
      executablePath?: string;
      extraFlags?: string;
    } & {
      [key: string]: unknown;
    };
    JavaRuntimeCapabilityDTO: {
      detectedMajor?: number;
      executablePath?: string;
      reason?: string;
      requiredMajor?: number;
      /** @description available, unavailable, or unknown; newer agents may add states. */
      state: string;
    } & {
      [key: string]: unknown;
    };
    JavaRuntimeDTO: {
      executablePath: string;
      majorVersion?: number;
      name: string;
    } & {
      [key: string]: unknown;
    };
    JavaRuntimeInstallRequestDTO: {
      /** @description One of JavaInstaller.minecraftInstallOptions' four majors (8/17/21/25, P7.7). OS/architecture are never client-supplied -- the agent always installs for its own host (D-009). */
      major: number;
    } & {
      [key: string]: unknown;
    };
    JavaRuntimeInstallResultDTO: {
      message: string;
      /** @description Always populated -- this route has no synchronous variant, same as WorldConvertResultDTO.operationId. */
      operationId: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    JavaRuntimesResponseDTO: {
      runtimes: components['schemas']['JavaRuntimeDTO'][];
    } & {
      [key: string]: unknown;
    };
    MeResponseDTO: {
      isNamedToken: boolean;
      name?: string;
      permissions?: string[];
      role: string;
    } & {
      [key: string]: unknown;
    };
    ModpackFileDTO: {
      /** @description Whether the pack manifest marks this file as client-only and MSC will skip it on the server. */
      clientOnly: boolean;
      path: string;
    } & {
      [key: string]: unknown;
    };
    ModpackImportRequestDTO: {
      /**
       * @description import: the active server is not yet pack-managed. replace: an explicit whole-pack replacement of an already pack-managed server. Sending import against an already pack-managed server, or replace against one that isn't, is refused as ambiguous (409 conflict) -- the client must know and say which case this is, never guessed server-side.
       * @enum {string}
       */
      action: 'import' | 'replace';
      stagedUploadId: string;
    } & {
      [key: string]: unknown;
    };
    ModpackImportResultDTO: {
      message: string;
      operationId: string;
      /** @description Populated once the operation reaches its blocked-file checkpoint (statusLine names it too); resolve each via POST /v1/modpacks/{operationId}/manual-file. Empty when the pack has no blocked files. */
      pendingManualFiles?: components['schemas']['ModpackManualFileEntryDTO'][];
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    ModpackInspectionResultDTO: {
      /** @description Files this pack's own precedence chain (manifest/Modrinth/embedded-jar/known-list, phase8-scope.md) will skip or disable -- reported so a client can show 'N files' vs 'N will actually install' before committing to import. */
      clientOnlyFileCount?: number;
      fileCount: number;
      /** @description Manifest files available for an mrpack inspection. CurseForge packs leave this empty because their file ids need provider resolution. */
      files: components['schemas']['ModpackFileDTO'][];
      /**
       * @description Unrecognized archives are a 400, not a 200 with a format value -- there is no third case to represent here.
       * @enum {string}
       */
      format: 'mrpack' | 'curseforge';
      loaderName?: string;
      loaderVersion?: string;
      /** @description CurseForge files this pack references whose authors block API distribution (D-027). Empty for an mrpack, or a CurseForge pack with no blocked files. */
      manualFiles: components['schemas']['ModpackManualFileEntryDTO'][];
      message: string;
      minecraftVersion?: string;
      /** @description Files supplied by the archive's overrides or server-overrides trees, counted separately from manifest downloads. */
      overrideFileCount: number;
      packName?: string;
      packVersion?: string;
      success: boolean;
      warnings?: string[];
    } & {
      [key: string]: unknown;
    };
    ModpackManualFileEntryDTO: {
      fileId: string;
      fileName: string;
      projectName?: string;
    } & {
      [key: string]: unknown;
    };
    ModpackManualFileRequestDTO: {
      fileId: string;
      stagedUploadId: string;
    } & {
      [key: string]: unknown;
    };
    ModpackManualFileResultDTO: {
      allFilesResolved: boolean;
      message: string;
      operationId: string;
      remainingManualFiles: components['schemas']['ModpackManualFileEntryDTO'][];
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    NotificationEventDTO: {
      body: string;
      helpId?: string | null;
      id: string;
      /** @enum {string} */
      kind:
        | 'server_started'
        | 'server_stopped'
        | 'player_joined'
        | 'player_left'
        | 'helper_failed'
        | 'connectivity_changed';
      occurredAtISO8601: string;
      serverId: string;
      title: string;
    } & {
      [key: string]: unknown;
    };
    /** @description Structured first-launch copy. Anchoring and presentation remain client-owned. */
    OnboardingGuideDTO: Record<string, never>;
    /** @description P2.5 SS4.1 -- POST /v1/operations request body. */
    OperationCreateRequestDTO: {
      /** @description Free-form, shape defined per type. */
      params?: Record<string, never> | null;
      target?: string | null;
      /** @description Required. Unrecognized values are a 400 invalid_body. */
      type: string;
    } & {
      [key: string]: unknown;
    };
    /** @description P2.5 SS2 -- the wire shape for a long-running operation. */
    OperationDTO: {
      /** @description Optional additive disclosure. False once a stop operation has entered graceful termination or another operation-specific non-cancelable boundary. */
      cancelable?: boolean;
      /** @description Present only when state == failed. */
      error?: components['schemas']['ErrorDTO'];
      /** @description Opaque, server-generated. Never client-supplied. */
      id: string;
      /** @description Null while queued, or for a type with no natural countable unit. */
      progress?:
        | ({
            current: number;
            total: number;
          } & {
            [key: string]: unknown;
          })
        | null;
      /** @description Present only when state == succeeded. Shape defined per type. */
      result?: Record<string, never> | null;
      /**
       * @description P2.5 SS3's closed state machine.
       * @enum {string}
       */
      state: 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';
      /** @description Human-readable, meant for direct display. */
      statusLine?: string | null;
      /** @description The thing the operation acts on, typically a server name/ID. Null if the operation type has no natural target. */
      target?: string | null;
      /** @description Kind of work, e.g. demo-install. Not a closed enum -- new values are additive (P2.5 SS2). */
      type: string;
    } & {
      [key: string]: unknown;
    };
    /** @description An administrator's requested browser or desktop grant. The resulting code is one-use and shown once. */
    PairingCreateRequestDTO: {
      /** @enum {string} */
      clientKind: 'browser' | 'desktop';
      /** @description Optional credential expiry. Pairing-code expiry is always ten minutes. */
      expiresAt?: string | null;
      label: string;
      permissions: string[];
      /** @enum {string} */
      role: 'admin' | 'guest' | 'named';
    } & {
      [key: string]: unknown;
    };
    PairingCreateResultDTO: {
      agentHostId: string;
      /** @enum {string} */
      clientKind: 'browser' | 'desktop';
      expiresAt: string;
      /** @description Raw 256-bit pairing code, returned only by POST /v1/auth/pairings. */
      pairingCode: string;
    } & {
      [key: string]: unknown;
    };
    /** @description P2.8's resolution of helpid-contract.md SS4's open item: PerformanceSnapshotDTO's bare scalars wrap into {value, helpId} (option b) rather than a separate static client-side name->helpId map (option a) -- consistent with every other helpId-bearing DTO in this contract attaching the pointer directly on the field's own object, at the cost of a DTO-shape change from the baseline. Proposed, pending Cameron's confirmation. */
    PerformanceMetricNumberDTO: {
      helpId?: string | null;
      value: number;
    } & {
      [key: string]: unknown;
    };
    PerformanceSnapshotDTO: {
      cpuPercent?: components['schemas']['PerformanceMetricNumberDTO'];
      playersOnline?: number;
      ramMaxMB?: components['schemas']['PerformanceMetricNumberDTO'];
      ramUsedMB?: components['schemas']['PerformanceMetricNumberDTO'];
      /** @description Optional runtime state for this performance source. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverType?: string;
      tps1m?: components['schemas']['PerformanceMetricNumberDTO'];
      /** @description Paper-family's real 5-minute rolling average from the same /tps reply as tps1m. Absent for single-value flavors (Forge/vanilla) and Bedrock. */
      tps5m?: components['schemas']['PerformanceMetricNumberDTO'];
      /** @description Paper-family's real 15-minute rolling average from the same /tps reply as tps1m. Absent for single-value flavors (Forge/vanilla) and Bedrock. */
      tps15m?: components['schemas']['PerformanceMetricNumberDTO'];
      ts: string;
      worldSizeMB?: components['schemas']['PerformanceMetricNumberDTO'];
    } & {
      [key: string]: unknown;
    };
    PlayerDeleteRequestDTO: {
      profileId: string;
    } & {
      [key: string]: unknown;
    };
    PlayerDTO: {
      name: string;
      uuid?: string;
    } & {
      [key: string]: unknown;
    };
    PlayerIdentifyRequestDTO: {
      gamertag: string;
      profileId: string;
    } & {
      [key: string]: unknown;
    };
    PlayerIdentifyResultDTO: {
      message: string;
      profileId?: string;
      success: boolean;
      username?: string;
    } & {
      [key: string]: unknown;
    };
    PlayerMigrateRequestDTO: {
      profileId: string;
      targetUuid: string;
    } & {
      [key: string]: unknown;
    };
    PlayerMutationResultDTO: {
      message: string;
      /** @description Set only by duplicate/migrate routes: the UUID the data now also lives under. */
      newProfileId?: string | null;
      profiles: components['schemas']['PlayerProfilesResponseDTO'];
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    PlayerProfileDTO: {
      hasSkinFileOverride?: boolean;
      id: string;
      imageIdentifier: string;
      inventory: components['schemas']['InventoryItemDTO'][];
      isBedrockPlayer: boolean;
      isHidden?: boolean;
      isOnline: boolean;
      isOp: boolean;
      lastSeen?: string;
      skinOverrideIdentifier?: string;
      stats?: components['schemas']['PlayerStatsDTO'];
      username?: string;
    } & {
      [key: string]: unknown;
    };
    PlayerProfilesResponseDTO: {
      isLoadingStats: boolean;
      profiles: components['schemas']['PlayerProfileDTO'][];
    } & {
      [key: string]: unknown;
    };
    PlayerSkinOverrideRequestDTO: {
      lookupIdentifier?: string;
      profileId: string;
    } & {
      [key: string]: unknown;
    };
    PlayerSkinOverrideResultDTO: {
      lookupIdentifier?: string;
      message: string;
      profileId?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    PlayerSkinResponseDTO: {
      imageBase64?: string;
      imageMimeType?: string;
      isOverride?: boolean;
      lookupIdentifier?: string;
      message: string;
      profileId?: string;
      source?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    PlayersResponseDTO: {
      count: number;
      note?: string;
      players: components['schemas']['PlayerDTO'][];
      /** @description Optional Bedrock runtime state for the player data source. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
    } & {
      [key: string]: unknown;
    };
    PlayerStatsDTO: {
      dimensionDisplay: string;
      foodLevel: number;
      gameMode: number;
      gameModeDisplay: string;
      health: number;
      maxHealth: number;
      posX: number;
      posY: number;
      posZ: number;
      score: number;
      xpLevel: number;
      xpTotal: number;
    } & {
      [key: string]: unknown;
    };
    PlayitActionResultDTO: {
      message?: string;
      /** @description Present when helper work continues; poll GET /v1/operations/{id} or stream its progress. */
      operationId?: string;
      result: string;
    } & {
      [key: string]: unknown;
    };
    /** @description Non-secret result of clearing host-local Playit state. Repeating reset is successful and returns already_clear once no local state remains. */
    PlayitResetResultDTO: {
      message?: string | null;
      operationId?: string | null;
      /** @enum {string} */
      result: 'cleared' | 'already_clear';
    } & {
      [key: string]: unknown;
    };
    /** @description Admission response for a native Playit setup operation. It contains no submitted credential, agent key, or provider session detail. */
    PlayitSetupAcceptedDTO: {
      message?: string | null;
      /** @description Opaque operation identifier for GET /v1/operations/{id} or the operation stream. */
      operationId: string;
      /** @enum {string} */
      result: 'setup_accepted';
    } & {
      [key: string]: unknown;
    };
    /** @description Native Playit sign-in input. Both fields are write-only at the API boundary and are held in memory only while the setup operation is running. */
    PlayitSetupRequestDTO: {
      email: string;
      password: string;
    } & {
      [key: string]: unknown;
    };
    PlayitStatusResponseDTO: {
      bedrockAddress?: string;
      hasSecretKey: boolean;
      isRunning: boolean;
      javaAddress?: string;
      note?: string;
      playitEnabled: boolean;
      serverName: string;
      serverType: string;
      voiceAddress?: string;
      voiceChatEnabled: boolean;
    } & {
      [key: string]: unknown;
    };
    RAMConfigResponseDTO: {
      hasActiveServer: boolean;
      maxRamGB: number;
      minRamGB: number;
      physicalRAMGB: number;
      recommendedMaxGB: number;
      serverName: string;
      serverRunning: boolean;
      serverType: string;
    } & {
      [key: string]: unknown;
    };
    RAMConfigUpdateRequestDTO: {
      maxRamGB?: number;
      minRamGB?: number;
    } & {
      [key: string]: unknown;
    };
    RAMConfigUpdateResultDTO: {
      maxRamGB?: number;
      message?: string;
      minRamGB?: number;
      restartRequired: boolean;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    RemoteAPIStatus: {
      activeServerId?: string;
      dockerContainerRunning?: boolean;
      dockerContainerStatus?: string;
      pid?: number;
      running: boolean;
      /** @description Optional runtime state for the active Bedrock server. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverType?: string;
    } & {
      [key: string]: unknown;
    };
    ResolvedRouterGuideDTO: {
      guide: components['schemas']['RouterGuideDTO'];
      runtime: components['schemas']['RouterRuntimeSummaryDTO'];
      sections: components['schemas']['RouterResolvedSectionDTO'][];
      unresolvedTokens: components['schemas']['RouterUnresolvedTokenDTO'][];
    } & {
      [key: string]: unknown;
    };
    ResourcePackActivateRequestDTO: {
      packId?: string;
      require?: boolean;
    } & {
      [key: string]: unknown;
    };
    ResourcePackItemDTO: {
      fileName: string;
      fileSizeDisplay: string;
      id: string;
      isActive: boolean;
      name: string;
      packKind: string;
      typeLabel: string;
    } & {
      [key: string]: unknown;
    };
    ResourcePackMutationResultDTO: {
      message: string;
      success: boolean;
      updated?: components['schemas']['ResourcePacksResponseDTO'];
    } & {
      [key: string]: unknown;
    };
    ResourcePackRemoveRequestDTO: {
      packId: string;
      packKind: string;
    } & {
      [key: string]: unknown;
    };
    ResourcePackSetURLRequestDTO: {
      require?: boolean;
      sha1?: string;
      url: string;
    } & {
      [key: string]: unknown;
    };
    ResourcePacksResponseDTO: {
      activePackUrl?: string;
      geyserPacks: components['schemas']['ResourcePackItemDTO'][];
      isGeyserAvailable: boolean;
      isJava: boolean;
      note?: string;
      packs: components['schemas']['ResourcePackItemDTO'][];
      requirePack: boolean;
      serverType: string;
    } & {
      [key: string]: unknown;
    };
    ResourcePackToggleRequestDTO: {
      enabled: boolean;
      packId: string;
    } & {
      [key: string]: unknown;
    };
    RouterAnalysisCauseDTO: {
      confidence: string;
      id: string;
      matchedSymptoms: string[];
      score: number;
      severity: string;
      topic: components['schemas']['RouterTroubleshootingTopicDTO'];
    } & {
      [key: string]: unknown;
    };
    RouterFallbackResolutionDTO: {
      availability: string;
      desiredFamily?: string | null;
      explanationBullets: string[];
      fallbackGuideId?: string | null;
      inferredFamilies: string[];
      kind: string;
      matchedGuideId?: string | null;
      matchedQuery?: string | null;
      recommendedNextNodeId?: string | null;
      suggestedSearchTerms: string[];
    } & {
      [key: string]: unknown;
    };
    RouterFallbackStateDTO: {
      networkType?: string | null;
      onlyKnowsIsp?: boolean;
      onlyKnowsMeshSystem?: boolean;
      searchQuery?: string | null;
      unsureWhetherIspOrOwnRouter?: boolean;
      wantsAdvancedTroubleshooting?: boolean;
    } & {
      [key: string]: unknown;
    };
    RouterGuideCatalogDTO: {
      guides: components['schemas']['RouterGuideDTO'][];
      symptoms: components['schemas']['RouterSymptomDTO'][];
      troubleshooting: components['schemas']['RouterTroubleshootingTopicDTO'][];
    } & {
      [key: string]: unknown;
    };
    RouterGuideDTO: {
      adminAddresses: string[];
      adminSurface: string;
      alternateMenuNames: string[];
      category: string;
      deviceDisplayName?: string | null;
      displayName: string;
      family: string;
      id: string;
      menuPath: string[];
      notes: components['schemas']['RouterGuideNoteDTO'][];
      providerDisplayName?: string | null;
      review: components['schemas']['RouterGuideReviewMetadataDTO'];
      searchKeywords: string[];
      sharedSections: components['schemas']['RouterGuideSharedSectionsDTO'];
      steps: components['schemas']['RouterGuideStepDTO'][];
      troubleshooting: string[];
    } & {
      [key: string]: unknown;
    };
    RouterGuideMatchCandidateDTO: {
      guide: components['schemas']['RouterGuideSummaryDTO'];
      reasons: string[];
      score: number;
    } & {
      [key: string]: unknown;
    };
    RouterGuideNoteDTO: {
      body: string;
      id: string;
      title?: string | null;
    } & {
      [key: string]: unknown;
    };
    RouterGuideReviewMetadataDTO: {
      lastReviewed?: string | null;
      reviewNotes?: string | null;
      sourceConfidence: string;
    } & {
      [key: string]: unknown;
    };
    RouterGuideSearchDTO: {
      candidates: components['schemas']['RouterGuideMatchCandidateDTO'][];
      fallbackResolution: components['schemas']['RouterFallbackResolutionDTO'];
      inferredFamilies: string[];
      isAmbiguous: boolean;
      matchedDirectGuide: boolean;
      normalizedQuery: string;
      normalizedTokens: string[];
      query: string;
      suggestedFallbackGuide?: components['schemas']['RouterGuideSummaryDTO'];
    } & {
      [key: string]: unknown;
    };
    RouterGuideSharedSectionsDTO: {
      includeSharedIntro: boolean;
      includeSharedPrerequisites: boolean;
      includeSharedTroubleshootingFooter: boolean;
      includeSharedValueSummary: boolean;
    } & {
      [key: string]: unknown;
    };
    RouterGuideStepDTO: {
      alternateTerms: string[];
      body: string;
      id: string;
      kind: string;
      referencedTokens: string[];
      title: string;
    } & {
      [key: string]: unknown;
    };
    RouterGuideSummaryDTO: {
      category: string;
      deviceDisplayName?: string | null;
      displayName: string;
      family: string;
      id: string;
      providerDisplayName?: string | null;
    } & {
      [key: string]: unknown;
    };
    RouterResolvedSectionDTO: {
      id: string;
      items: Record<string, never>[];
      kind: string;
      origin: string;
      title: string;
    } & {
      [key: string]: unknown;
    };
    RouterRuntimeSummaryDTO: {
      bedrockEnabled: boolean | null;
      bedrockPort: number | null;
      detectedGatewayIpAddress: string | null;
      detectedLocalIpAddress: string | null;
      javaPort: number | null;
      recommendedProtocol: string | null;
      selectedServerId: string | null;
      selectedServerName: string | null;
    } & {
      [key: string]: unknown;
    };
    RouterSymptomDTO: {
      description: string;
      id: string;
      title: string;
    } & {
      [key: string]: unknown;
    };
    RouterTroubleshootingAnalyzeRequestDTO: {
      fallbackState?: components['schemas']['RouterFallbackStateDTO'];
      symptoms: string[];
    } & {
      [key: string]: unknown;
    };
    RouterTroubleshootingAnalyzeResponseDTO: {
      escalationBullets: string[];
      fallbackResolution?: components['schemas']['RouterFallbackResolutionDTO'];
      likelyCauses: components['schemas']['RouterAnalysisCauseDTO'][];
      recommendedActions: string[];
      summary: string;
      symptoms: string[];
    } & {
      [key: string]: unknown;
    };
    RouterTroubleshootingTopicDTO: {
      id: string;
      suggestedNextActions: string[];
      summary: string;
      title: string;
    } & {
      [key: string]: unknown;
    };
    RouterUnresolvedTokenDTO: {
      sectionId: string;
      token: string;
    } & {
      [key: string]: unknown;
    };
    ServerCreateRequestDTO: {
      acceptEula?: boolean;
      /** @description Requested BDS release for serverType=bedrock. The agent resolves and verifies the platform-appropriate distribution entry. */
      bedrockVersion?: string;
      /** @description Acknowledgement token returned in a confirmation_required error for a Creative or other safety-sensitive world choice. */
      confirmation?: string;
      crossPlayBedrockPort?: number;
      difficulty?: string;
      dockerImage?: string;
      enableCrossPlay?: boolean;
      enablePlayit?: boolean;
      /** @description Creation-time intent from a staged Simple Voice Chat add-on. The agent creates the voice tunnel only after an active SVC plugin or mod is present in plugins/ or mods/. */
      enableVoiceChat?: boolean;
      enableXboxBroadcast?: boolean;
      gamemode?: string;
      javaFlavor?: string;
      javaPath?: string;
      loaderVersion?: string;
      maxPlayers?: number;
      minecraftVersion?: string;
      name: string;
      port?: number;
      serverType?: string;
      /** @description A staged upload (purpose modpack-archive) already inspected via POST /v1/modpacks/inspect. When present, provisioning pins the loader/Minecraft version from the pack and applies its mod list as part of this same create operation (P8.21) -- MSC 1 has no separate create-from-pack primitive; this mirrors createNewServer + applyStagedAddOn composed into one durable operation instead of two. */
      stagedModpackUploadId?: string;
      versionId?: string;
      worldName?: string;
      worldSeed?: string;
      /** @description Optional complete profile for the first fresh world. The agent applies it before the server's first start. */
      worldSettings?: components['schemas']['ServerCreateWorldSettingsDTO'];
    } & {
      [key: string]: unknown;
    };
    ServerCreateResultDTO: {
      message: string;
      /** @description Phase 4 lifecycle operation id for progress polling or /v1/operations/{id}/stream; optional so older clients can ignore it. */
      operationId?: string;
      /** @description Optional Bedrock runtime state. Older clients may ignore this additive field. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverId?: string;
      serverName?: string;
      success: boolean;
      warnings?: string[];
    } & {
      [key: string]: unknown;
    };
    /** @description Creation-time world profile. Safety and readback metadata are produced by the agent after the world slot exists. */
    ServerCreateWorldSettingsDTO: {
      gameplay?: components['schemas']['WorldGameplayDTO'];
      generation?: components['schemas']['WorldGenerationDTO'];
      identity?: components['schemas']['WorldIdentityDTO'];
    } & {
      [key: string]: unknown;
    };
    ServerDeleteRequestDTO: {
      serverId: string;
    } & {
      [key: string]: unknown;
    };
    ServerDeleteResultDTO: {
      message: string;
      serverId?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    ServerDirectoryRequestDTO: {
      directory: string;
      serverId: string;
    } & {
      [key: string]: unknown;
    };
    ServerDirectoryResultDTO: {
      directory?: string;
      message: string;
      serverId?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    ServerDirectorySizeResponseDTO: {
      serverId: string;
      /** Format: int64 */
      sizeBytes?: number;
    } & {
      [key: string]: unknown;
    };
    ServerDTO: {
      /** @description Configured Java cross-play Bedrock port, when present. */
      bedrockPort?: number;
      directory: string;
      /** @description True when the next server start must run the two-pass first-start initiation flow. */
      firstStartRequired?: boolean;
      gamePort?: number;
      hostAddress?: string;
      id: string;
      javaFlavor?: string;
      name: string;
      /** @description Whether Playit is enabled for this server. */
      playitEnabled?: boolean;
      /** @description Optional current Bedrock runtime state for an imported or created server. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverType: string;
      /** @description Whether Xbox Broadcast is enabled for this server. */
      xboxBroadcastEnabled?: boolean;
    } & {
      [key: string]: unknown;
    };
    ServerEULARequestDTO: {
      serverId?: string;
    } & {
      [key: string]: unknown;
    };
    ServerEULAResultDTO: {
      accepted?: boolean;
      message: string;
      serverId?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    ServerFileItemDTO: {
      fileExtension?: string;
      id: string;
      isDirectory: boolean;
      isPreviewable?: boolean;
      modifiedAt?: string;
      name: string;
      path: string;
      sizeBytes?: number;
    } & {
      [key: string]: unknown;
    };
    ServerFileReadResponseDTO: {
      content?: string;
      encoding?: string;
      message: string;
      name?: string;
      path?: string;
      sizeBytes?: number;
      success: boolean;
      truncated?: boolean;
    } & {
      [key: string]: unknown;
    };
    ServerFilesResponseDTO: {
      items: components['schemas']['ServerFileItemDTO'][];
      note?: string;
      parentPath?: string;
      path: string;
      serverName?: string;
    } & {
      [key: string]: unknown;
    };
    ServerImportRequestDTO: {
      acceptEula?: boolean;
      /** @enum {string} */
      action: 'scan' | 'importExisting' | 'importTransfer' | 'rescan';
      activeWorldName?: string;
      backupPath?: string;
      bedrockPortOverrides?: {
        [key: string]: number;
      };
      displayName?: string;
      enablePlayit?: boolean;
      importKind?: string;
      javaPortOverrides?: {
        [key: string]: number;
      };
      maxPlayers?: number;
      port?: number;
      serverType?: string;
      /** @description Required for scan/importExisting/importTransfer; omitted for rescan, which scans the configured servers root. */
      sourcePath?: string;
      transferMode?: string;
    } & {
      [key: string]: unknown;
    };
    ServerImportResultDTO: {
      imported?: number;
      message: string;
      operationId: string;
      replaced?: boolean;
      /** @description Optional Bedrock runtime state for the imported record. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      serverId?: string;
      serverName?: string;
      skipped?: number;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    ServerImportScanResponseDTO: {
      defaultWorldName?: string;
      detectedLoaderVersion?: string;
      detectedMCVersion?: string;
      eulaAccepted?: boolean;
      isZip?: boolean;
      javaFlavor?: string;
      maxPlayers?: number;
      message: string;
      port?: number;
      serverType?: string;
      sourcePath?: string;
      success: boolean;
      worlds?: components['schemas']['ServerImportWorldDTO'][];
    } & {
      [key: string]: unknown;
    };
    ServerImportWorldDTO: {
      dimensionsLabel: string;
      id: string;
      name: string;
      sizeBytes: number;
    } & {
      [key: string]: unknown;
    };
    ServerRenameRequestDTO: {
      name: string;
      serverId: string;
    } & {
      [key: string]: unknown;
    };
    ServerRenameResultDTO: {
      message: string;
      name?: string;
      serverId?: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    ServersRootResponseDTO: {
      path: string;
    } & {
      [key: string]: unknown;
    };
    ServersRootSetRequestDTO: {
      path: string;
    } & {
      [key: string]: unknown;
    };
    SessionEventDTO: {
      eventType: string;
      id: string;
      playerName: string;
      timestamp: string;
    } & {
      [key: string]: unknown;
    };
    SessionLogResponseDTO: {
      activeServerId?: string;
      events: components['schemas']['SessionEventDTO'][];
    } & {
      [key: string]: unknown;
    };
    SettingFieldDTO: {
      /** @description Replaces the baseline's free-text help field (helpid-contract.md SS4) -- resolves via GET /v1/help/{helpId}. */
      helpId?: string | null;
      key: string;
      label: string;
      maxInt?: number;
      maxLength?: number;
      minInt?: number;
      options?: components['schemas']['SettingOptionDTO'][];
      type: string;
      unit?: string;
      value: string;
    } & {
      [key: string]: unknown;
    };
    SettingOptionDTO: {
      label: string;
      value: string;
    } & {
      [key: string]: unknown;
    };
    SettingRejectionDTO: {
      key: string;
      reason: string;
    } & {
      [key: string]: unknown;
    };
    SettingsResponseDTO: {
      editable: boolean;
      note?: string;
      /** @description Optional runtime state; settings can be readable before a runtime is runnable. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      sections: components['schemas']['SettingsSectionDTO'][];
      serverName: string;
      serverRunning: boolean;
      serverType: string;
    } & {
      [key: string]: unknown;
    };
    SettingsSectionDTO: {
      fields: components['schemas']['SettingFieldDTO'][];
      icon: string;
      id: string;
      title: string;
    } & {
      [key: string]: unknown;
    };
    SettingsUpdateRequestDTO: {
      changes: {
        [key: string]: string;
      };
      /** @description Acknowledgement token returned in a confirmation_required error before applying the server-wide gamemode override. */
      confirmation?: string;
    } & {
      [key: string]: unknown;
    };
    SettingsUpdateResultDTO: {
      appliedKeys: string[];
      message: string;
      rejected?: components['schemas']['SettingRejectionDTO'][];
      restartRequired: boolean;
      /** @description Optional Bedrock runtime state after settings are applied. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      sections?: components['schemas']['SettingsSectionDTO'][];
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    SimpleResult: {
      activeServerId?: string;
      /** @description Phase 4 lifecycle operation id for progress polling or /v1/operations/{id}/stream; optional so older clients can ignore it. */
      operationId?: string;
      result: string;
      /** @description Optional Bedrock runtime state associated with the lifecycle request. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
    } & {
      [key: string]: unknown;
    };
    StagedUploadBeginRequestDTO: {
      contentType?: string;
      /** @description Required, curseforge-manual-file only: which of the operation's pending blocked files this upload is for. */
      fileId?: string;
      /** @description Required, curseforge-manual-file only: the pending modpack-import operation this upload resumes. The agent looks up the expected file's own reported byte size/name from CurseForge's file metadata already recorded against that operation -- maxBytes on the result is sized to that exact file, not a flat ceiling. */
      operationId?: string;
      /** @enum {string} */
      purpose:
        | 'world-import'
        | 'active-world-replace'
        | 'world-thumbnail'
        | 'modpack-archive'
        | 'addon-local-file'
        | 'curseforge-manual-file';
    } & {
      [key: string]: unknown;
    };
    StagedUploadBeginResultDTO: {
      expiresAt: string;
      maxBytes: number;
      stagedUploadId: string;
      /** @description PUT /v1/staged-uploads/{id} -- bounded to this token, not an arbitrary remote path. */
      uploadPath: string;
    } & {
      [key: string]: unknown;
    };
    StagedUploadCompleteResultDTO: {
      receivedBytes: number;
      sha256: string;
      stagedUploadId: string;
    } & {
      [key: string]: unknown;
    };
    StartupProblemDTO: {
      availableActions: string[];
      /** @description Keyed off kind, e.g. diagnostics.crash.forge-dep (helpid-contract.md SS4). */
      helpId?: string | null;
      iconSystemName: string;
      id: string;
      installedFile?: string;
      installedJarStem?: string;
      isRepairing: boolean;
      kind: string;
      kindTitle: string;
      missingDependency?: string;
      modrinthURL?: string;
      offenderName: string;
      rawExcerpt: string;
      requirement?: string;
    } & {
      [key: string]: unknown;
    };
    TemplateItemDTO: {
      build?: number;
      displayName: string;
      filename: string;
      id: string;
      kind: string;
      modifiedAt?: string;
      sizeBytes?: number;
      version?: string;
    } & {
      [key: string]: unknown;
    };
    TemplateMutationRequestDTO: {
      acceptEula?: boolean;
      action: string;
      crossPlayBedrockPort?: number;
      difficulty?: string;
      enableCrossPlay?: boolean;
      enablePlayit?: boolean;
      gamemode?: string;
      includePlugins?: boolean;
      name?: string;
      port?: number;
      serverId?: string;
      templateId?: string;
      worldName?: string;
      worldSeed?: string;
    } & {
      [key: string]: unknown;
    };
    TemplateMutationResultDTO: {
      createdServerId?: string;
      createdServerName?: string;
      exportedCount?: number;
      message: string;
      success: boolean;
      templates?: components['schemas']['TemplatesResponseDTO'];
    } & {
      [key: string]: unknown;
    };
    TemplatesResponseDTO: {
      note?: string;
      paperTemplates: components['schemas']['TemplateItemDTO'][];
      pluginTemplates: components['schemas']['TemplateItemDTO'][];
      serverName?: string;
      serverRunning: boolean;
    } & {
      [key: string]: unknown;
    };
    ThirdPartyWorldConfigBoundaryDTO: {
      available: boolean;
      /** @enum {string} */
      handoff: 'server_settings';
      helpId?: string;
      label: string;
      message: string;
    } & {
      [key: string]: unknown;
    };
    UserCreateRequestDTO: {
      expiresInDays?: number;
      label: string;
      permissions?: string[];
      role: string;
    } & {
      [key: string]: unknown;
    };
    UserCreateResultDTO: {
      message: string;
      success: boolean;
      token?: string;
      user?: components['schemas']['UserSummaryDTO'];
    } & {
      [key: string]: unknown;
    };
    UserListResponseDTO: {
      users: components['schemas']['UserSummaryDTO'][];
    } & {
      [key: string]: unknown;
    };
    UserRevokeRequestDTO: {
      userId: string;
    } & {
      [key: string]: unknown;
    };
    UserRevokeResultDTO: {
      message: string;
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    UserSummaryDTO: {
      createdAtISO8601?: string;
      expiresAtISO8601?: string;
      id: string;
      isExpired: boolean;
      label: string;
      permissions?: string[];
      role: string;
    } & {
      [key: string]: unknown;
    };
    UserUpdateRequestDTO: {
      expiresInDays?: number;
      label?: string;
      permissions?: string[];
      role?: string;
      userId: string;
    } & {
      [key: string]: unknown;
    };
    UserUpdateResultDTO: {
      message: string;
      success: boolean;
      user?: components['schemas']['UserSummaryDTO'];
    } & {
      [key: string]: unknown;
    };
    VersionChangeRequestDTO: {
      loaderVersion?: string;
      versionId: string;
    } & {
      [key: string]: unknown;
    };
    VersionChangeResultDTO: {
      message: string;
      /** @description Phase 7 addition (P7.9): operation id for progress polling or /v1/operations/{id}/stream and cancellation; optional so older clients can ignore it, same pattern as ServerCreateResultDTO.operationId. */
      operationId?: string;
      requiresRestart: boolean;
      /** @description Optional Bedrock runtime state after a version request. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    VersionEntryDTO: {
      buildLabel?: string;
      displayLabel: string;
      id: string;
      isLatest: boolean;
      isStable: boolean;
      loaderVersion?: string;
      mcVersion: string;
    } & {
      [key: string]: unknown;
    };
    VersionsResponseDTO: {
      currentVersion?: string;
      flavorName: string;
      isBedrock: boolean;
      note?: string;
      /** @description Optional Bedrock runtime state. */
      runtime?: components['schemas']['BedrockRuntimeStateDTO'];
      supportsVersions: boolean;
      versions: components['schemas']['VersionEntryDTO'][];
    } & {
      [key: string]: unknown;
    };
    WatchdogActionErrorDTO: {
      error: string;
      success: string;
    } & {
      [key: string]: unknown;
    };
    WatchdogActionResultDTO: {
      success: string;
    } & {
      [key: string]: unknown;
    };
    WatchdogStatusResponseDTO: {
      enabled: string;
    } & {
      [key: string]: unknown;
    };
    WorldActivateRequestDTO: {
      /** @description Acknowledgement token returned in a confirmation_required error before activating a safety-sensitive world. */
      confirmation?: string;
      slotId: string;
    } & {
      [key: string]: unknown;
    };
    WorldActivateResultDTO: {
      /** @description Operation id for progress polling (GET /v1/operations/{id}) or /v1/operations/{id}/stream and cancellation; optional so older clients can ignore it, matching SimpleResult's P4 precedent. */
      operationId?: string;
      result: string;
    } & {
      [key: string]: unknown;
    };
    WorldConvertFormatsResponseDTO: {
      /** @description Raw format strings reported by the installed Chunker jar, including both JAVA_ and BEDROCK_ formats. */
      formats: string[];
    } & {
      [key: string]: unknown;
    };
    /** @description Corrected post-review: MSC 1 conversion always names a separate, opposite-edition target server (AppViewModel+WorldConversion.swift's own sourceServer/targetServer parameters) -- sourceSlotId resolves against the active server; targetServerId is a separate, required server id. targetFormat is the exact Chunker format string the client chose from the installed jar's own supported list (MSC 1's wizard defaults its picker to the newest compatible format but always lets the user override it -- never hardcoded server-side). Exactly one of targetName (place into a fresh slot) or targetSlotId (overwrite an existing slot on the target server, by id, not display name) must be present. */
    WorldConvertRequestDTO: {
      sourceSlotId: string;
      targetFormat: string;
      targetName?: string;
      targetServerId: string;
      targetSlotId?: string;
    } & {
      [key: string]: unknown;
    };
    /** @description Always operation-backed (type: world-conversion, operation-model.md SS2) -- Chunker's process lifetime makes this the one Phase 6 world mutation with no synchronous variant. */
    WorldConvertResultDTO: {
      operationId: string;
      result: string;
    } & {
      [key: string]: unknown;
    };
    WorldCreateRequestDTO: {
      /** @description Acknowledgement token returned in a confirmation_required error before applying a safety-sensitive world choice. */
      confirmation?: string;
      name: string;
      seed?: string;
    } & {
      [key: string]: unknown;
    };
    WorldDeleteRequestDTO: {
      slotId: string;
    } & {
      [key: string]: unknown;
    };
    WorldDuplicateRequestDTO: {
      slotId: string;
    } & {
      [key: string]: unknown;
    };
    WorldExportRequestDTO: {
      slotId: string;
    } & {
      [key: string]: unknown;
    };
    WorldExportResultDTO: {
      expiresAt: string;
      sizeBytes: number;
      stagedDownloadId: string;
    } & {
      [key: string]: unknown;
    };
    /** @description Persistent gameplay choices for the selected world. Gamerules, experiments, and supported toggles are open maps so newer keys are not silently discarded. */
    WorldGameplayDTO: {
      /** @description Bedrock cheat state. */
      cheats?: boolean | null;
      /** @description Java allow-commands state for the world. */
      commands?: boolean | null;
      coordinates?: boolean | null;
      defaultGameMode?: string | null;
      difficulty?: string | null;
      experiments: {
        [key: string]: boolean;
      };
      gamerules: {
        [key: string]: string;
      };
      hardcore?: boolean | null;
      startingMap?: boolean | null;
      /** @description Edition-specific gameplay toggles advertised by the agent. */
      supportedToggles: {
        [key: string]: boolean;
      };
    } & {
      [key: string]: unknown;
    };
    /** @description World generation choices. Creation-only values are retained even when the generated world can no longer safely change them. */
    WorldGenerationDTO: {
      biomeSource?: string | null;
      bonusChest?: boolean | null;
      dataPacks: string[];
      flatPreset?: string | null;
      /** @description Opaque generator-options payload retained for round-trip safety. */
      generatorOptions?: string | null;
      structures?: boolean | null;
      worldType?: string | null;
    } & {
      [key: string]: unknown;
    };
    /** @description Identity that belongs to one slot, not to every server world. */
    WorldIdentityDTO: {
      /** @description Minecraft level/folder name used when the slot is active. */
      levelName?: string | null;
      /** @description MSC display name for the slot. */
      name?: string | null;
      /** @description Seed used for generation when this world is first created. */
      seed?: string | null;
    } & {
      [key: string]: unknown;
    };
    WorldImportRequestDTO: {
      backupId?: string;
      name: string;
      stagedUploadId?: string;
    } & {
      [key: string]: unknown;
    };
    WorldMutationResultDTO: {
      message: string;
      success: boolean;
      updated?: components['schemas']['WorldSlotsResponseDTO'];
    } & {
      [key: string]: unknown;
    };
    WorldProfileChangeDTO: {
      key: string;
      reason?: string | null;
      /** @enum {string} */
      status: 'live' | 'pending_restart' | 'blocked';
    } & {
      [key: string]: unknown;
    };
    /** @description The world-local source of truth for one WorldSlot. This object is versioned independently from the server profile so a client can degrade when a runtime adds a field it does not know. */
    WorldProfileDTO: {
      /** @description Metadata keyed by stable profile field key. It tells clients which capability gates the field, when a change takes effect, how confidently the value was read, and where to find the explanation. */
      fieldMetadata: {
        [key: string]: components['schemas']['WorldProfileFieldMetadataDTO'];
      };
      gameplay: components['schemas']['WorldGameplayDTO'];
      generation: components['schemas']['WorldGenerationDTO'];
      identity: components['schemas']['WorldIdentityDTO'];
      safety: components['schemas']['WorldSafetyDTO'];
      /** @description World profile schema version. Version 1 is the initial MSC 2 shape. */
      schemaVersion: number;
    } & {
      [key: string]: unknown;
    };
    /** @description Capability and teaching metadata for one world-profile field. */
    WorldProfileFieldMetadataDTO: {
      /** @description Capability key supplied by the agent for this field. */
      capability: string;
      /** @description Agent-served explanation resolved through GET /v1/help/{helpId}. */
      helpId?: string | null;
      /** @enum {string} */
      lifecycle: 'creation_only' | 'apply_on_activation' | 'live_safe' | 'restart_required';
      /** @enum {string} */
      valueState: 'configured' | 'detected' | 'unknown' | 'unsupported' | 'achievement_disabled';
    } & {
      [key: string]: unknown;
    };
    /** @description Sparse updates to one slot-local world profile. Keys use the stable dotted profile names; a nested profile section is also accepted for client convenience. */
    WorldProfileUpdateRequestDTO: ({
      /** @description Stable dotted world profile field keys and JSON values. */
      changes?: {
        [key: string]: unknown;
      };
      /** @description Acknowledgement token returned in a confirmation_required error before applying a safety-sensitive profile change. */
      confirmation?: string;
      profile?: components['schemas']['WorldProfileDTO'];
    } & {
      [key: string]: unknown;
    }) &
      (unknown | unknown);
    /** @description The saved slot profile plus the status of each requested projection change. */
    WorldProfileUpdateResultDTO: {
      changes: components['schemas']['WorldProfileChangeDTO'][];
      message: string;
      slot: components['schemas']['WorldSlotWithProfileDTO'];
      /** @enum {string} */
      status: 'live' | 'pending_restart' | 'blocked';
      success: boolean;
    } & {
      [key: string]: unknown;
    };
    /** @description Direct rename of the active/live world's on-disk folders (AppViewModel+WorldManagement.swift's renameWorld) -- distinct from WorldRenameRequestDTO, which renames a slot's metadata only and touches no files. */
    WorldRenameActiveWorldRequestDTO: {
      name: string;
    } & {
      [key: string]: unknown;
    };
    WorldRenameRequestDTO: {
      name: string;
      slotId: string;
    } & {
      [key: string]: unknown;
    };
    WorldRepairRequestDTO: {
      slotId: string;
    } & {
      [key: string]: unknown;
    };
    WorldRepairResultDTO: {
      /** @description Operation id for progress polling (GET /v1/operations/{id}) or /v1/operations/{id}/stream and cancellation; optional so older clients can ignore it, matching the operation-backed world activation response. */
      operationId?: string;
      result: string;
    } & {
      [key: string]: unknown;
    };
    /** @description AppViewModel+WorldManagement.swift's replaceWorld -- direct live-world replacement. Separately named from WorldReplaceRequestDTO (WorldSlotManager.copySlotIntoExisting, a saved-slot-to-saved-slot copy that never touches the live world); see phase6-api.md SS9/SS10. */
    WorldReplaceActiveRequestDTO: {
      newLevelName: string;
      /** @description Redeems a staged upload begun with purpose "active-world-replace" as the replacement world's source ZIP. Omit to replace with a fresh (empty) world instead -- never an arbitrary server-local path. */
      stagedUploadId?: string;
    } & {
      [key: string]: unknown;
    };
    /** @description Always operation-backed: the mandatory pre-replace safety backup and (for an uploaded source) zip staging are real filesystem work, the same class as activate/backups-restore. */
    WorldReplaceActiveResultDTO: {
      /** @description Operation id for progress polling (GET /v1/operations/{id}) or /v1/operations/{id}/stream and cancellation; optional so older clients can ignore it, matching SimpleResult's P4 precedent. */
      operationId?: string;
      result: string;
    } & {
      [key: string]: unknown;
    };
    WorldReplaceRequestDTO: {
      slotId: string;
      sourceSlotId: string;
    } & {
      [key: string]: unknown;
    };
    /** @description Detected safety state for the world. Known state values are safe, achievement_disabled, unknown, and unsupported. */
    WorldSafetyDTO: {
      reasons: string[];
      state: string;
    } & {
      [key: string]: unknown;
    };
    WorldSettingCapabilityDTO: {
      available: boolean;
      capability: string;
      helpId?: string;
      reason?: string;
      state: string;
    } & {
      [key: string]: unknown;
    };
    WorldSettingsCapabilitiesDTO: {
      context: components['schemas']['WorldSettingsContextDTO'];
      fields: {
        [key: string]: components['schemas']['WorldSettingCapabilityDTO'];
      };
      thirdParty: components['schemas']['ThirdPartyWorldConfigBoundaryDTO'];
    } & {
      [key: string]: unknown;
    };
    WorldSettingsContextDTO: {
      javaFlavor?: string;
      javaRuntime?: components['schemas']['JavaRuntimeCapabilityDTO'];
      loaderVersion?: string;
      minecraftVersion?: string;
      nativeCapabilities: string[];
      /** @enum {string} */
      serverType: 'java' | 'bedrock';
    } & {
      [key: string]: unknown;
    };
    /** @description Legacy slot summary. A detailed slot response pairs this summary with WorldProfileDTO; P12.24 begins emitting the profile after safe metadata migration. */
    WorldSlotDTO: {
      createdAt: string;
      hasThumbnail: boolean;
      id: string;
      isActive: boolean;
      name: string;
      worldSeed?: string;
      zipSizeBytes?: number;
    } & {
      [key: string]: unknown;
    };
    WorldSlotsResponseDTO: {
      activeSlotId?: string;
      isRepairing?: boolean;
      serverRunning: boolean;
      slots: components['schemas']['WorldSlotDTO'][];
    } & {
      [key: string]: unknown;
    };
    /** @description A world slot together with the versioned profile that travels with it. The profile is slot-owned; it is not duplicated in SettingsResponseDTO. */
    WorldSlotWithProfileDTO: {
      profile: components['schemas']['WorldProfileDTO'];
      slot: components['schemas']['WorldSlotDTO'];
    } & {
      [key: string]: unknown;
    };
    WorldThumbnailUploadRequestDTO: {
      stagedUploadId: string;
    } & {
      [key: string]: unknown;
    };
  };
  responses: never;
  parameters: never;
  requestBodies: never;
  headers: never;
  pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
  exchangeBrowserSession: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['BrowserSessionExchangeRequestDTO'];
      };
    };
    responses: {
      /** @description Session created; sets the httpOnly msc2_session cookie */
      204: {
        headers: {
          [name: string]: unknown;
        };
        content?: never;
      };
      /** @description invalid_body */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description forbidden (wrong Origin) */
      403: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description pairing_consumed */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description pairing_expired */
      410: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description rate_limited */
      429: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  logoutBrowserSession: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description Session revoked and msc2_session cleared */
      204: {
        headers: {
          [name: string]: unknown;
        };
        content?: never;
      };
      /** @description unauthorized */
      401: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description forbidden (wrong Origin or missing/bad X-MSC-CSRF) */
      403: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  getCsrfToken: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description CSRF token; Cache-Control is no-store */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['CsrfTokenResponseDTO'];
        };
      };
    };
  };
  exchangeDesktopPairing: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['DesktopPairingExchangeRequestDTO'];
      };
    };
    responses: {
      /** @description Credential created; token is returned once to the Tauri backend */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['DesktopCredentialResultDTO'];
        };
      };
      /** @description invalid_body */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description pairing_consumed */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description pairing_expired */
      410: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description rate_limited */
      429: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  createPairing: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['PairingCreateRequestDTO'];
      };
    };
    responses: {
      /** @description Pairing code created; the code is shown only in this response */
      201: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['PairingCreateResultDTO'];
        };
      };
      /** @description invalid_body */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description forbidden (non-admin credential) */
      403: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description rate_limited */
      429: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  deleteBackup: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['BackupDeleteRequestDTO'];
      };
    };
    responses: {
      /** @description Backup deleted */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['SimpleResult'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description sole_verified_backup -- Phase 6's retention-floor correction: MSC 1's count-based pruning has no floor against deleting the last remaining verified backup (fixtures/backups), this route does. */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  createBackupNow: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description Backup started */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['BackupNowResultDTO'];
        };
      };
    };
  };
  restoreBackup: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['BackupRestoreRequestDTO'];
      };
    };
    responses: {
      /** @description Restore started */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['BackupRestoreResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing_backup_id */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description backup_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  installComponent: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['CatalogInstallRequestDTO'];
      };
    };
    responses: {
      /** @description Install admitted and started (including a local-JAR install via stagedUploadId). success/message/count/installedDependencies land on the operation's terminal result; a client that ignores operationId and only reads the body still gets a truthful in-flight message, not a final result. */
      202: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['CatalogInstallResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing_project */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description not_supported / no_active_server / pack_managed (ErrorDTO.details carries packName/packVersion) */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  changeVersion: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['VersionChangeRequestDTO'];
      };
    };
    responses: {
      /** @description Version change started */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['VersionChangeResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing_version_id */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description server_running / no_active_server / not_supported */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description download_in_progress */
      429: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  resetHost: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['HostResetRequestDTO'];
      };
    };
    responses: {
      /** @description Reset accepted as a journaled operation. The old credential may stop working while the agent restarts or changes to a needs-pairing/unavailable state. */
      202: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['HostResetAcceptedDTO'];
        };
      };
      /** @description invalid_body, missing_mode, invalid_mode, missing_confirmation, or confirmation_mismatch */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description forbidden (administrator credential required) */
      403: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description server_running or reset_in_progress */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal_error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  installJavaRuntime: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['JavaRuntimeInstallRequestDTO'];
      };
    };
    responses: {
      /** @description Install started */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['JavaRuntimeInstallResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / invalid_major */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description download_in_progress (an install for this major is already running) */
      429: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  completeModpackManualFile: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        operationId: string;
      };
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['ModpackManualFileRequestDTO'];
      };
    };
    responses: {
      /** @description File bound and verified */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ModpackManualFileResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing_file_id / missing_staged_upload_id */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description unknown operationId, operation not a pending modpack-import, unknown fileId, or staged upload not found/expired/wrong purpose */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description wrong file for this fileId (hash/size/name mismatch against CurseForge's own recorded metadata -- ErrorDTO.details carries expectedFileId/expectedFileName/expectedByteSize) */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  importModpack: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['ModpackImportRequestDTO'];
      };
    };
    responses: {
      /** @description Import admitted and started */
      202: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ModpackImportResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing_staged_upload_id / missing_action / invalid_archive / missing_curseforge_api_key */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description staged upload not found, expired, or not purpose modpack-archive */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / pack_managed (action=import against an already pack-managed server, or action=replace against one that isn't -- ErrorDTO.details carries packName/packVersion) */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  inspectModpack: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': {
          stagedUploadId: string;
        } & {
          [key: string]: unknown;
        };
      };
    };
    responses: {
      /** @description Inspection result */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ModpackInspectionResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / unrecognized_archive / malformed_manifest / unsafe_archive_path */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description staged upload not found, expired, or not purpose modpack-archive */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  resetPlayit: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description Host-local state cleared or already clear */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['PlayitResetResultDTO'];
        };
      };
      /** @description forbidden (networking permission required) */
      403: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description playit_reset_failed */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description playit_reset_failed while persisting cleared state */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  setupPlayit: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['PlayitSetupRequestDTO'];
      };
    };
    responses: {
      /** @description Native setup accepted; operationId is populated and progress is available from GET /v1/operations/{id} or its stream. */
      202: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['PlayitSetupAcceptedDTO'];
        };
      };
      /** @description invalid_json / missing_email / missing_password */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description forbidden (networking permission required) */
      403: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description setup_in_progress / setup_unavailable / credential_store_failed / tunnel_mismatch / public_addresses_unavailable */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  createServer: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['ServerCreateRequestDTO'];
      };
    };
    responses: {
      /** @description Server creation started (bedrock: capability_unavailable is returned as 409 instead, see x-notes) */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ServerCreateResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / name_required / invalid_server_type / invalid_java_flavor / unsupported_server_type */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description create_failed / capability_unavailable (Bedrock create, refused rather than half-provisioned; Phase 10) */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  downloadStagedBytes: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description Staged file bytes */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/zip': string;
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  beginStagedUpload: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['StagedUploadBeginRequestDTO'];
      };
    };
    responses: {
      /** @description Staging slot created */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['StagedUploadBeginResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  uploadStagedBytes: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/octet-stream': string;
      };
    };
    responses: {
      /** @description Upload accepted */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['StagedUploadCompleteResultDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description staged_upload_expired / max_bytes_exceeded */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  getWorldSlotProfile: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        slotId: string;
      };
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description World slot and its profile */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldSlotWithProfileDTO'];
        };
      };
      /** @description no_active_server / slot_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  updateWorldSlotProfile: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        slotId: string;
      };
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldProfileUpdateRequestDTO'];
      };
    };
    responses: {
      /** @description Profile saved with per-field application status */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldProfileUpdateResultDTO'];
        };
      };
      /** @description invalid_body / invalid_json */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / world_reconciliation_degraded */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  getWorldSlotThumbnail: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description Thumbnail image bytes */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'image/jpeg': string;
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  setWorldSlotThumbnail: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        slotId: string;
      };
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldThumbnailUploadRequestDTO'];
      };
    };
    responses: {
      /** @description Mutation applied */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldMutationResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / invalid_body */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / world_reconciliation_degraded */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  activateWorldSlot: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldActivateRequestDTO'];
      };
    };
    responses: {
      /** @description Activation started */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldActivateResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing_slot_id */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description server_running_or_slot_not_found */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  convertWorld: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldConvertRequestDTO'];
      };
    };
    responses: {
      /** @description Conversion started */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldConvertResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / server_running / active_slot_refused / sole_verified_backup */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  getWorldConvertFormats: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description Supported Chunker conversion formats */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldConvertFormatsResponseDTO'];
        };
      };
      /** @description capability_unavailable when Java or Chunker is not installed */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  deleteWorldSlot: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldDeleteRequestDTO'];
      };
    };
    responses: {
      /** @description Mutation applied */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldMutationResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / server_running / active_slot_refused / sole_verified_backup */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  duplicateWorldSlot: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldDuplicateRequestDTO'];
      };
    };
    responses: {
      /** @description Mutation applied */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldMutationResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / server_running / active_slot_refused / sole_verified_backup */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  exportWorldSlot: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldExportRequestDTO'];
      };
    };
    responses: {
      /** @description Staged download prepared */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldExportResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / server_running / active_slot_refused / sole_verified_backup */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  importWorldSlot: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldImportRequestDTO'];
      };
    };
    responses: {
      /** @description Mutation applied */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldMutationResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / server_running / active_slot_refused / sole_verified_backup */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  renameActiveWorld: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldRenameActiveWorldRequestDTO'];
      };
    };
    responses: {
      /** @description Mutation applied */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldMutationResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / server_running / active_slot_refused / sole_verified_backup */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  replaceActiveWorld: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        'application/json': components['schemas']['WorldReplaceActiveRequestDTO'];
      };
    };
    responses: {
      /** @description Replacement started */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldReplaceActiveResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / name_required */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description unknown or already-redeemed stagedUploadId */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description server_running / conflict */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
  updateActiveWorldSlot: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      /** @description Mutation applied */
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['WorldMutationResultDTO'];
        };
      };
      /** @description missing_body / invalid_json / missing field */
      400: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description slot_not_found / backup_not_found / staged_upload_not_found */
      404: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description no_active_server / server_running / active_slot_refused / sole_verified_backup */
      409: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
      /** @description internal error */
      500: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          'application/json': components['schemas']['ErrorDTO'];
        };
      };
    };
  };
}

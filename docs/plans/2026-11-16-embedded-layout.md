# Embedded Ledger Layout

**Goal:** Remove the competing inner sidebar and make the plugin work as a native content panel inside the host navigation, released as0.2.1.

**Design:** One content column, horizontal functional tabs and compact language/reload controls. The host supplies the page title and application navigation; the plugin retains an accessible section heading without another large workspace header. Inherit the host background, typography and primary color, remove the dark brand rail and oversized welcome artwork, and reduce inner padding. Keep charts, reporting dimensions, bilingual controls and all financial behavior.

**Implementation:** Update `App.vue` and scoped CSS, bump plugin/backend/frontend patch versions, refresh README images and installation filenames. Build/typecheck/lint the frontend and run browser interactions against the real Linux plugin under the simulated SDK host. Capture an explicitly labelled simulated host sidebar/title around the iframe to verify the composition at desktop and narrow widths. Rebuild/package Linux and verify manifests/resources before publishing a new release.

**Acceptance:** No `aside` or inner app sidebar in the plugin; full available content width; functional tabs stay reachable in both languages; no page-level horizontal overflow at480px or when embedded beside a250px host sidebar; language switching, queries and downloads still work. Screenshots must label fictional data and the simulated host shell.

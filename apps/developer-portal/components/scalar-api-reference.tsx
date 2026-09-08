"use client";

import { ApiReferenceReact } from "@scalar/api-reference-react";
import "@scalar/api-reference-react/style.css";
import { apiBaseUrl } from "@/lib/web-app";

/**
 * Full-bleed Scalar OpenAPI reference themed to Altius-FTM tokens.
 */
export function ScalarApiReference() {
  const servers = [
    { url: `${apiBaseUrl()}/api/v3`, description: "Configured API base" },
  ];

  return (
    <div className="scalar-app min-h-[calc(100vh-3.5rem)]">
      <ApiReferenceReact
        configuration={{
          url: "/openapi/altius-ftm-v3.openapi.yaml",
          theme: "default",
          darkMode: false,
          hideDarkModeToggle: true,
          servers,
          metaData: {
            title: "Altius-FTM API",
          },
        }}
      />
    </div>
  );
}

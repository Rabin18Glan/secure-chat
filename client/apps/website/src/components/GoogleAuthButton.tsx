import React, { useState, useEffect } from "react";

interface GoogleAuthButtonProps {
  onSuccess: (user: any) => void;
  onError: (errorMsg: string) => void;
}

declare global {
  interface Window {
    google?: {
      accounts: {
        id: {
          initialize: (config: {
            client_id: string;
            callback: (response: { credential: string }) => void;
            auto_select?: boolean;
          }) => void;
          renderButton: (
            parent: HTMLElement,
            options: {
              theme?: "outline" | "filled_blue" | "filled_black";
              size?: "large" | "medium" | "small";
              width?: number;
              text?: string;
              shape?: "rectangular" | "pill" | "circle";
            },
          ) => void;
          prompt: () => void;
        };
      };
    };
  }
}

export const GoogleAuthButton: React.FC<GoogleAuthButtonProps> = ({ onSuccess, onError }) => {
  const [clientId, setClientId] = useState<string>(
    () => import.meta.env.VITE_GOOGLE_CLIENT_ID || localStorage.getItem("google_client_id") || "",
  );
  const [showConfig, setShowConfig] = useState(false);
  const [customToken, setCustomToken] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  // Load client ID from .env or backend config
  useEffect(() => {
    if (!clientId) {
      fetch("/api/auth/google/config")
        .then((res) => res.json())
        .then((data) => {
          if (data.client_id) {
            setClientId(data.client_id);
          }
        })
        .catch((err) => console.warn("Could not fetch Google config:", err));
    }

    // Check if returning from Google OAuth redirect with ?code=
    const params = new URLSearchParams(window.location.search);
    const code = params.get("code");
    if (code) {
      // Clear code from URL without reload
      const state = params.get("state") || undefined;
      params.delete("code");
      params.delete("state");
      const query = params.toString();
      window.history.replaceState(
        {},
        document.title,
        window.location.pathname + (query ? "?" + query : "") + window.location.hash,
      );
      void handleGoogleCredentialResponse({ code, state });
    }
  }, []);

  // Initialize Google Identity Services (GIS)
  useEffect(() => {
    if (!clientId) return;

    const checkAndInitGoogle = () => {
      if (window.google?.accounts?.id) {
        try {
          window.google.accounts.id.initialize({
            client_id: clientId,
            callback: (resp) => handleGoogleCredentialResponse({ credential: resp.credential }),
          });

          const buttonElement = document.getElementById("google-signin-btn");
          if (buttonElement) {
            buttonElement.innerHTML = "";
            window.google.accounts.id.renderButton(buttonElement, {
              theme: "outline",
              size: "large",
              shape: "rectangular",
              width: 320,
            });
          }
        } catch (e) {
          console.error("GIS initialization error:", e);
        }
      }
    };

    checkAndInitGoogle();
    const interval = setInterval(() => {
      if (window.google?.accounts?.id) {
        checkAndInitGoogle();
        clearInterval(interval);
      }
    }, 400);

    return () => clearInterval(interval);
  }, [clientId]);

  const handleGoogleCredentialResponse = async (payload: {
    credential?: string;
    code?: string;
    state?: string;
  }) => {
    setIsLoading(true);
    try {
      const res = await fetch("/api/auth/google", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const data = await res.json();
      if (data.status && data.user) {
        onSuccess(data.user);
      } else {
        onError(data.msg || data.message || "Google authentication failed");
      }
    } catch {
      onError("Network error during Google authentication");
    } finally {
      setIsLoading(false);
    }
  };

  const handleRedirectLogin = async () => {
    try {
      setIsLoading(true);
      const res = await fetch("/api/auth/google/url");
      const data = await res.json();
      if (data.url) {
        window.location.href = data.url;
      } else {
        onError("Google OAuth URL not configured");
      }
    } catch {
      onError("Failed to initiate Google OAuth redirect");
    } finally {
      setIsLoading(false);
    }
  };

  const handleManualTokenSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!customToken.trim()) return;
    await handleGoogleCredentialResponse({ credential: customToken.trim() });
  };

  const saveClientId = (id: string) => {
    setClientId(id);
    localStorage.setItem("google_client_id", id);
    setShowConfig(false);
  };

  return (
    <div className="w-full space-y-3">
      <div className="relative flex items-center justify-center">
        <div className="border-t border-slate-200 w-full"></div>
        <span className="bg-white px-3 text-xs uppercase tracking-wider text-slate-500 font-semibold absolute">
          Or continue with
        </span>
      </div>

      <div className="pt-2 flex flex-col items-center justify-center">
        <div id="google-signin-btn" className="w-full flex justify-center min-h-[44px]">
          {/* Default fallback if Google script is loading or rendering */}
          <button
            type="button"
            onClick={handleRedirectLogin}
            className="w-full flex items-center justify-center gap-3 py-2.5 px-4 rounded-xl bg-white border border-slate-200 hover:border-slate-200 text-sm font-medium text-slate-700 transition-all  hover:bg-white"
          >
            <svg className="w-5 h-5" viewBox="0 0 24 24">
              <path
                fill="#4285F4"
                d="M23.745 12.27c0-.7-.06-1.4-.19-2.07H12v4.51h6.6c-.29 1.52-1.14 2.82-2.4 3.68v3.05h3.88c2.27-2.09 3.665-5.17 3.665-9.17Z"
              />
              <path
                fill="#34A853"
                d="M12 24c3.24 0 5.95-1.08 7.93-2.91l-3.88-3.05c-1.08.72-2.45 1.16-4.05 1.16-3.12 0-5.77-2.1-6.72-4.93H1.25v3.15C3.26 21.36 7.33 24 12 24Z"
              />
              <path
                fill="#FBBC05"
                d="M5.28 14.27c-.25-.72-.38-1.49-.38-2.27s.13-1.55.38-2.27V6.58H1.25C.45 8.18 0 9.99 0 12s.45 3.82 1.25 5.42l4.03-3.15Z"
              />
              <path
                fill="#EA4335"
                d="M12 4.75c1.77 0 3.35.61 4.6 1.8l3.42-3.42C17.95 1.19 15.24 0 12 0 7.33 0 3.26 2.64 1.25 6.58l4.03 3.15c.95-2.83 3.6-4.98 6.72-4.98Z"
              />
            </svg>
            <span>Sign in with Google</span>
          </button>
        </div>

        {isLoading && (
          <p className="text-xs text-blue-600 animate-pulse mt-2">
            Verifying Google identity token...
          </p>
        )}

        <div className="mt-2 flex items-center justify-between w-full px-1">
          <button
            type="button"
            onClick={() => setShowConfig(!showConfig)}
            className="text-[11px] text-slate-500 hover:text-slate-500 underline underline-offset-2 transition-colors"
          >
            {showConfig ? "Hide Google Config" : "Google OAuth Details"}
          </button>

          <span className="text-[10px] text-slate-600 font-mono">
            {clientId ? `ID: ...${clientId.slice(-16)}` : "Reading from .env"}
          </span>
        </div>

        {showConfig && (
          <div className="mt-3 w-full p-4 rounded-xl bg-white border border-slate-200 space-y-3 text-xs animate-in fade-in duration-150">
            <p className="font-semibold text-slate-700">Active Google OAuth Environment</p>
            <div className="space-y-1">
              <label className="text-slate-500 font-medium">Active Client ID</label>
              <input
                type="text"
                value={clientId}
                onChange={(e) => setClientId(e.target.value)}
                className="w-full bg-white border border-slate-200 rounded-lg px-3 py-1.5 text-slate-700 focus:outline-none focus:border-blue-200 font-mono text-xs"
              />
            </div>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => saveClientId(clientId)}
                className="flex-1 bg-blue-600 hover:bg-blue-700 text-white font-bold py-1.5 rounded-lg transition-colors"
              >
                Save
              </button>
              <button
                type="button"
                onClick={() => setShowConfig(false)}
                className="px-3 bg-slate-100 hover:bg-slate-100 text-slate-700 rounded-lg"
              >
                Close
              </button>
            </div>

            <div className="pt-2 border-t border-slate-200">
              <label className="text-slate-500 font-medium block mb-1">Direct ID Token Test:</label>
              <div className="flex gap-2">
                <input
                  type="text"
                  placeholder="Paste Google JWT ID token"
                  value={customToken}
                  onChange={(e) => setCustomToken(e.target.value)}
                  className="flex-1 bg-white border border-slate-200 rounded-lg px-3 py-1.5 text-slate-700 focus:outline-none focus:border-blue-200 font-mono text-xs"
                />
                <button
                  type="button"
                  onClick={handleManualTokenSubmit}
                  className="px-3 bg-emerald-600 hover:bg-emerald-500 text-slate-900 font-medium rounded-lg"
                >
                  Verify
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

import React, { useState } from "react";
import { Video, Plus, ArrowRight, ShieldCheck, Lock, Sparkles, KeyRound } from "lucide-react";
import { generateSessionKey } from "../utils/crypto";

interface User {
  _id: string;
  username: string;
  email: string;
  avatarImage: string;
}

interface LobbyProps {
  currentUser: User;
  onEnterMeeting: (sessionId: string, sessionKey: string) => void;
}

export const Lobby: React.FC<LobbyProps> = ({ currentUser, onEnterMeeting }) => {
  const [meetingTitle, setMeetingTitle] = useState("");
  const [joinInput, setJoinInput] = useState("");
  const [manualKey, setManualKey] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  const handleCreateMeeting = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMessage("");
    setIsCreating(true);

    try {
      const res = await fetch("/api/sessions/create", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: meetingTitle.trim() || `${currentUser.username}'s Secure Meeting`,
          ownerId: currentUser._id,
        }),
      });

      const data = await res.json();
      if (res.ok && data.sessionId) {
        // Generate cryptographic key client-side
        const key = generateSessionKey();
        onEnterMeeting(data.sessionId, key);
      } else {
        setErrorMessage(data.msg || data.message || "Failed to create meeting");
      }
    } catch {
      setErrorMessage("Network error while creating session");
    } finally {
      setIsCreating(false);
    }
  };

  const handleJoinMeeting = (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMessage("");

    const input = joinInput.trim();
    if (!input) return;

    let targetSessionId = "";
    let targetKey = manualKey.trim();

    try {
      // Check if user pasted a full URL
      if (
        input.startsWith("http://") ||
        input.startsWith("https://") ||
        input.includes("?session=")
      ) {
        const url = new URL(input, window.location.origin);
        const sParam = url.searchParams.get("session");
        if (sParam) {
          targetSessionId = sParam;
        }
        if (url.hash) {
          targetKey = url.hash.replace("#", "");
        }
      } else {
        // Just the session ID
        targetSessionId = input;
      }
    } catch {
      targetSessionId = input;
    }

    if (!targetSessionId) {
      setErrorMessage("Please enter a valid session ID or invite link.");
      return;
    }

    if (!/^[a-f0-9]{64}$/i.test(targetKey)) {
      setErrorMessage("Use a full invite link or enter the 64-character session key.");
      return;
    }

    onEnterMeeting(targetSessionId, targetKey);
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center p-6 bg-white overflow-y-auto">
      <div className="w-full max-w-4xl space-y-8 py-8">
        {/* Hero Title */}
        <div className="text-center space-y-3">
          <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full bg-blue-50 border border-blue-200 text-blue-600 text-xs font-semibold">
            <Sparkles className="w-4 h-4" />
            <span>Your meeting space</span>
          </div>
          <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-slate-900">
            Meet. Talk. Stay connected.
          </h1>
          <p className="text-sm sm:text-base text-slate-500 max-w-2xl mx-auto">
            Start a video meeting or join your team with an invite link. You decide who comes in.
          </p>
        </div>

        {errorMessage && (
          <div className="max-w-md mx-auto p-3.5 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-600 text-xs text-center font-medium">
            {errorMessage}
          </div>
        )}

        {/* Action Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Create Meeting Card */}
          <div className="bg-white border border-slate-200 hover:border-slate-200 rounded-2xl p-6 sm:p-8 space-y-6 flex flex-col justify-between transition-all  ">
            <div className="space-y-4">
              <div className="w-12 h-12 rounded-2xl bg-blue-50 border border-blue-200 flex items-center justify-center text-blue-600">
                <Video className="w-6 h-6" />
              </div>
              <div>
                <h3 className="text-lg font-bold text-slate-900">New Meeting</h3>
                <p className="text-xs text-slate-500 mt-1">
                  Bring everyone together. Start a call and invite your team.
                </p>
              </div>

              <form onSubmit={handleCreateMeeting} className="space-y-4 pt-2">
                <div>
                  <label className="block text-xs font-semibold text-slate-500 mb-1.5">
                    Meeting Title (Optional)
                  </label>
                  <input
                    type="text"
                    value={meetingTitle}
                    onChange={(e) => setMeetingTitle(e.target.value)}
                    placeholder="e.g. Confidential Project Review"
                    className="w-full bg-white border border-slate-200 focus:border-blue-200 rounded-xl px-4 py-2.5 text-xs text-slate-900 placeholder-slate-500 focus:outline-none transition-colors"
                  />
                </div>

                <button
                  type="submit"
                  disabled={isCreating}
                  className="w-full flex items-center justify-center gap-2 py-3 px-4 rounded-xl bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white font-bold text-xs transition-all  "
                >
                  <Plus className="w-4 h-4" />
                  <span>{isCreating ? "Generating Session..." : "Start Instant Meeting"}</span>
                </button>
              </form>
            </div>

            <div className="pt-4 border-t border-slate-200 flex items-center gap-2 text-[11px] text-slate-500">
              <ShieldCheck className="w-4 h-4 text-emerald-700 shrink-0" />
              <span>Meeting key created on your device</span>
            </div>
          </div>

          {/* Join Meeting Card */}
          <div className="bg-white border border-slate-200 hover:border-slate-200 rounded-2xl p-6 sm:p-8 space-y-6 flex flex-col justify-between transition-all  ">
            <div className="space-y-4">
              <div className="w-12 h-12 rounded-2xl bg-indigo-500/10 border border-indigo-500/30 flex items-center justify-center text-indigo-600">
                <KeyRound className="w-6 h-6" />
              </div>
              <div>
                <h3 className="text-lg font-bold text-slate-900">Join Meeting</h3>
                <p className="text-xs text-slate-500 mt-1">
                  Enter with an invite link or meeting code. If you are not an admin, you will wait
                  in the waiting room until admitted.
                </p>
              </div>

              <form onSubmit={handleJoinMeeting} className="space-y-4 pt-2">
                <div>
                  <label className="block text-xs font-semibold text-slate-500 mb-1.5">
                    Meeting Code or Invite Link
                  </label>
                  <input
                    type="text"
                    required
                    value={joinInput}
                    onChange={(e) => setJoinInput(e.target.value)}
                    placeholder="e.g. meet-8a21f7 or full invite URL"
                    className="w-full bg-white border border-slate-200 focus:border-blue-500 rounded-xl px-4 py-2.5 text-xs text-slate-900 placeholder-slate-500 focus:outline-none transition-colors"
                  />
                </div>

                <div>
                  <label className="block text-xs font-semibold text-slate-500 mb-1.5">
                    Session key (optional)
                  </label>
                  <input
                    type="text"
                    value={manualKey}
                    onChange={(e) => setManualKey(e.target.value)}
                    placeholder="Optional 64-character hex key"
                    className="w-full bg-white border border-slate-200 focus:border-blue-500 rounded-xl px-4 py-2.5 text-xs text-slate-900 placeholder-slate-500 focus:outline-none transition-colors font-mono"
                  />
                </div>

                <button
                  type="submit"
                  disabled={!joinInput.trim()}
                  className="w-full flex items-center justify-center gap-2 py-3 px-4 rounded-xl bg-slate-100 hover:bg-slate-100 disabled:opacity-40 text-slate-700 font-bold text-xs transition-all border border-slate-200"
                >
                  <span>Join meeting</span>
                  <ArrowRight className="w-4 h-4" />
                </button>
              </form>
            </div>

            <div className="pt-4 border-t border-slate-200 flex items-center gap-2 text-[11px] text-slate-500">
              <Lock className="w-4 h-4 text-blue-600 shrink-0" />
              <span>Admin approval required for regular members</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

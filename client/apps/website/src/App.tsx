import React, { useState, useEffect, useRef } from "react";
import {
  Send,
  LogOut,
  MessageSquare,
  Shield,
  Smile,
  User as UserIcon,
  Video,
  Sparkles,
} from "lucide-react";
import { GoogleAuthButton } from "./components/GoogleAuthButton";
import { Lobby } from "./components/Lobby";
import { MeetingRoom } from "./components/MeetingRoom";
import { generateSessionKey } from "./utils/crypto";

interface User {
  _id: string;
  username: string;
  email: string;
  avatarImage: string;
  auth_provider?: string;
}

interface DirectMessage {
  fromSelf: boolean;
  message: string;
}

export const App: React.FC = () => {
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [activeTab, setActiveTab] = useState<"chat" | "meeting">("chat");

  // Meeting State
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [activeSessionKey, setActiveSessionKey] = useState<string | null>(null);

  // Direct 1-to-1 Chat State
  const [contacts, setContacts] = useState<User[]>([]);
  const [selectedChat, setSelectedChat] = useState<User | null>(null);
  const [messages, setMessages] = useState<DirectMessage[]>([]);
  const [textInput, setTextInput] = useState("");

  // Auth form state
  const [isLoginMode, setIsLoginMode] = useState(true);
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [errorMessage, setErrorMessage] = useState("");

  const directSocketRef = useRef<WebSocket | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const selectedChatRef = useRef<User | null>(null);
  useEffect(() => {
    selectedChatRef.current = selectedChat;
  }, [selectedChat]);

  // Read saved session or URL invitation on mount
  useEffect(() => {
    fetch("/api/auth/me")
      .then(async (res) => {
        if (!res.ok) {
          localStorage.removeItem("chat-app-user");
          return;
        }
        const user = await res.json();
        setCurrentUser(user);
        localStorage.setItem("chat-app-user", JSON.stringify(user));
      })
      .catch(() => localStorage.removeItem("chat-app-user"));

    // Check if user came in via meeting invite link (?session=meet-xxx#key=yyy)
    const searchParams = new URLSearchParams(window.location.search);
    const sessionParam = searchParams.get("session");
    if (sessionParam) {
      const hashKey = window.location.hash.replace("#", "");
      setActiveSessionId(sessionParam);
      if (/^[a-f0-9]{64}$/i.test(hashKey)) {
        setActiveSessionKey(hashKey);
      } else {
        setErrorMessage("This meeting invite is missing a valid session key.");
      }
      setActiveTab("meeting");
    }
  }, []);

  // Fetch contacts and open direct chat socket
  useEffect(() => {
    if (!currentUser) return;

    fetch(`/api/auth/allusers/${currentUser._id}`)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setContacts(data);
        }
      })
      .catch((err) => console.error("Error fetching users:", err));

    // Connect to WebSocket for direct messaging
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host;
    const ws = new WebSocket(`${protocol}//${host}/ws?userId=${currentUser._id}`);

    ws.onopen = () => {
      ws.send(JSON.stringify({ event: "add-user", userId: currentUser._id }));
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.event === "msg-recieve" && data.from === selectedChatRef.current?._id) {
          setMessages((prev) => [...prev, { fromSelf: false, message: data.msg }]);
        }
      } catch (err) {
        console.error("WS error:", err);
      }
    };

    directSocketRef.current = ws;

    return () => {
      ws.close();
    };
  }, [currentUser]);

  // Fetch direct messages when a contact is selected
  useEffect(() => {
    if (!currentUser || !selectedChat) return;

    setMessages([]);
    const controller = new AbortController();
    fetch("/api/messages/getmsg", {
      signal: controller.signal,
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ from: currentUser._id, to: selectedChat._id }),
    })
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setMessages(data);
        }
      })
      .catch((err) => {
        if (err.name !== "AbortError") console.error("Error fetching messages:", err);
      });
    return () => controller.abort();
  }, [currentUser, selectedChat]);

  // Scroll to bottom on new direct message
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Handle standard login/register
  const handleAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMessage("");

    const endpoint = isLoginMode ? "/api/auth/login" : "/api/auth/register";
    const payload = isLoginMode ? { username, password } : { username, email, password };

    try {
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const data = await res.json();

      if (data.status && data.user) {
        setCurrentUser(data.user);
        localStorage.setItem("chat-app-user", JSON.stringify(data.user));
      } else {
        setErrorMessage(data.msg || data.message || "Authentication failed");
      }
    } catch {
      setErrorMessage("Network error, please check backend connection.");
    }
  };

  const handleLogout = async () => {
    if (currentUser) {
      await fetch(`/api/auth/logout/${currentUser._id}`).catch(() => {});
    }
    localStorage.removeItem("chat-app-user");
    setCurrentUser(null);
    setActiveSessionId(null);
    setActiveSessionKey(null);
    setContacts([]);
    setSelectedChat(null);
    setMessages([]);
    directSocketRef.current?.close();
  };

  const handleSendDirectMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!textInput.trim() || !currentUser || !selectedChat) return;

    const msg = textInput.trim();
    setTextInput("");

    try {
      const res = await fetch("/api/messages/addmsg", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ from: currentUser._id, to: selectedChat._id, message: msg }),
      });
      if (!res.ok) {
        const error = await res.json();
        throw new Error(error.msg || error.message || "Could not save message");
      }
      if (selectedChatRef.current?._id === selectedChat._id) {
        setMessages((prev) => [...prev, { fromSelf: true, message: msg }]);
      }
      if (directSocketRef.current?.readyState === WebSocket.OPEN) {
        directSocketRef.current.send(
          JSON.stringify({ event: "send-msg", to: selectedChat._id, msg }),
        );
      }
    } catch (error) {
      setTextInput(msg);
      setErrorMessage(error instanceof Error ? error.message : "Could not send message");
    }
  };

  const handleEnterMeeting = (sessionId: string, sessionKey: string) => {
    setActiveSessionId(sessionId);
    setActiveSessionKey(sessionKey);
    // Update browser URL without reloading so the fragment #key stays safe
    const newUrl = `${window.location.pathname}?session=${sessionId}#${sessionKey}`;
    window.history.pushState(null, "", newUrl);
  };

  const handleLeaveMeeting = () => {
    setActiveSessionId(null);
    setActiveSessionKey(null);
    window.history.pushState(null, "", window.location.pathname);
  };

  /* ---------------- Auth Screen ---------------- */
  if (!currentUser) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-white p-4 text-slate-700">
        <div className="w-full max-w-md rounded-2xl bg-white border border-slate-200 p-8 sm:p-10">
          <div className="flex items-center justify-center gap-3 mb-6">
            <div className="p-3 bg-blue-50 rounded-2xl border border-blue-200">
              <Shield className="h-8 w-8 text-blue-600" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight text-slate-900">
                Secure Chat & Meet
              </h1>
              <p className="text-xs text-slate-500">A simple space to connect.</p>
            </div>
          </div>

          <div className="flex rounded-xl bg-white p-1 mb-6 border border-slate-200">
            <button
              onClick={() => setIsLoginMode(true)}
              className={`flex-1 py-2 text-xs font-semibold rounded-lg transition-all ${
                isLoginMode
                  ? "bg-blue-600 text-white font-bold"
                  : "text-slate-500 hover:text-slate-900"
              }`}
            >
              Sign In
            </button>
            <button
              onClick={() => setIsLoginMode(false)}
              className={`flex-1 py-2 text-xs font-semibold rounded-lg transition-all ${
                !isLoginMode
                  ? "bg-blue-600 text-white font-bold"
                  : "text-slate-500 hover:text-slate-900"
              }`}
            >
              Register
            </button>
          </div>

          {errorMessage && (
            <div className="mb-4 rounded-xl bg-rose-500/10 border border-rose-500/30 p-3 text-xs text-rose-600">
              {errorMessage}
            </div>
          )}

          <div className="mb-6">
            <h2 className="text-2xl font-semibold tracking-tight text-slate-900">
              {isLoginMode ? "Welcome back" : "Create an account"}
            </h2>
            <p className="mt-2 text-sm text-slate-500">
              {isLoginMode
                ? "Sign in to your conversations and meetings."
                : "Get started with chat and video meetings."}
            </p>
          </div>
          <form onSubmit={handleAuth} className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-slate-500 mb-1.5">Username</label>
              <input
                type="text"
                required
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="johndoe"
                className="w-full rounded-xl bg-white border border-slate-200 px-4 py-2.5 text-xs text-slate-900 placeholder-slate-500 focus:outline-none focus:border-blue-200 transition-colors"
              />
            </div>

            {!isLoginMode && (
              <div>
                <label className="block text-xs font-semibold text-slate-500 mb-1.5">Email</label>
                <input
                  type="email"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="john@example.com"
                  className="w-full rounded-xl bg-white border border-slate-200 px-4 py-2.5 text-xs text-slate-900 placeholder-slate-500 focus:outline-none focus:border-blue-200 transition-colors"
                />
              </div>
            )}

            <div>
              <label className="block text-xs font-semibold text-slate-500 mb-1.5">Password</label>
              <input
                type="password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••••••"
                className="w-full rounded-xl bg-white border border-slate-200 px-4 py-2.5 text-xs text-slate-900 placeholder-slate-500 focus:outline-none focus:border-blue-200 transition-colors"
              />
            </div>

            <button
              type="submit"
              className="w-full mt-2 rounded-xl bg-blue-600 hover:bg-blue-700 py-3 text-xs font-bold text-white transition-colors  "
            >
              {isLoginMode ? "Sign In" : "Create Account"}
            </button>
          </form>

          {/* Google OAuth Login */}
          <div className="mt-5">
            <GoogleAuthButton
              onSuccess={(user) => {
                setCurrentUser(user);
                localStorage.setItem("chat-app-user", JSON.stringify(user));
              }}
              onError={(err) => setErrorMessage(err)}
            />
          </div>
        </div>
      </div>
    );
  }

  /* ---------------- Active Meeting View ---------------- */
  if (activeSessionId && activeSessionKey) {
    return (
      <MeetingRoom
        currentUser={currentUser}
        sessionId={activeSessionId}
        sessionKey={activeSessionKey}
        onLeave={handleLeaveMeeting}
      />
    );
  }

  /* ---------------- Main Dashboard (Direct Chat & Meeting Lobby) ---------------- */
  return (
    <div className="flex h-screen bg-white text-slate-700 overflow-hidden">
      {errorMessage && (
        <div
          role="alert"
          className="absolute top-3 left-1/2 -translate-x-1/2 z-50 rounded-xl border border-rose-200 bg-white px-4 py-3 text-sm text-rose-600"
        >
          {errorMessage}
          <button className="ml-4 underline" onClick={() => setErrorMessage("")}>
            Dismiss
          </button>
        </div>
      )}
      {/* Sidebar */}
      <div className="w-60 lg:w-72 shrink-0 border-r border-slate-200 bg-white flex flex-col">
        {/* User Profile Bar */}
        <div className="p-4 border-b border-slate-200 flex items-center justify-between">
          <div className="flex items-center gap-3 min-w-0">
            <div className="p-2 bg-blue-50 rounded-xl border border-blue-200 shrink-0">
              <Shield className="h-5 w-5 text-blue-600" />
            </div>
            <div className="min-w-0">
              <h2 className="text-sm font-bold text-slate-900 truncate">{currentUser.username}</h2>
              <p className="text-xs text-emerald-700 flex items-center gap-1.5">
                <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                Online
              </p>
            </div>
          </div>
          <button
            onClick={handleLogout}
            title="Log Out"
            className="p-2 text-slate-500 hover:text-rose-600 hover:bg-rose-500/10 rounded-xl transition-colors shrink-0"
          >
            <LogOut className="h-4 w-4" />
          </button>
        </div>

        {/* Tab Switcher: Direct Chat vs Meetings */}
        <div className="p-3 border-b border-slate-200">
          <div className="grid grid-cols-2 gap-1.5 bg-white p-1 rounded-xl border border-slate-200 text-xs font-semibold">
            <button
              onClick={() => setActiveTab("chat")}
              className={`flex items-center justify-center gap-2 py-2 rounded-lg transition-all ${
                activeTab === "chat"
                  ? "bg-blue-600 text-white font-bold"
                  : "text-slate-500 hover:text-slate-900"
              }`}
            >
              <MessageSquare className="w-4 h-4" />
              <span>Direct Chat</span>
            </button>
            <button
              onClick={() => setActiveTab("meeting")}
              className={`flex items-center justify-center gap-2 py-2 rounded-lg transition-all ${
                activeTab === "meeting"
                  ? "bg-blue-600 text-white font-bold"
                  : "text-slate-500 hover:text-slate-900"
              }`}
            >
              <Video className="w-4 h-4" />
              <span>Meetings</span>
            </button>
          </div>
        </div>

        {/* Sidebar Content */}
        {activeTab === "chat" ? (
          <>
            <div className="px-4 py-2 flex items-center justify-between">
              <p className="text-[11px] font-bold uppercase tracking-wider text-slate-500">
                Contacts ({contacts.length})
              </p>
            </div>

            <div className="flex-1 overflow-y-auto px-2 space-y-1">
              {contacts.map((contact) => {
                const isSelected = selectedChat?._id === contact._id;
                return (
                  <button
                    key={contact._id}
                    onClick={() => setSelectedChat(contact)}
                    className={`w-full flex items-center gap-3 p-3 rounded-2xl transition-all ${
                      isSelected
                        ? "bg-blue-50 border border-blue-200 text-slate-900"
                        : "hover:bg-slate-100 text-slate-700"
                    }`}
                  >
                    <div className="h-10 w-10 rounded-xl bg-slate-100 border border-slate-200 flex items-center justify-center shrink-0">
                      <UserIcon className="h-5 w-5 text-slate-500" />
                    </div>
                    <div className="text-left overflow-hidden">
                      <p className="text-sm font-semibold truncate">{contact.username}</p>
                      <p className="text-xs text-slate-500 truncate">{contact.email}</p>
                    </div>
                  </button>
                );
              })}
              {contacts.length === 0 && (
                <p className="text-xs text-slate-500 text-center py-8">No other contacts found.</p>
              )}
            </div>
          </>
        ) : (
          <div className="p-4 space-y-4">
            <div className="p-4 rounded-2xl bg-white border border-slate-200 text-xs space-y-2">
              <div className="flex items-center gap-2 text-blue-600 font-bold">
                <Sparkles className="w-4 h-4" />
                <span>Google Meet Style</span>
              </div>
              <p className="text-slate-500 leading-relaxed">
                Host or join meetings with knocking approval, E2EE key in the URL hash, and admin
                role controls.
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Main Workspace Area */}
      {activeTab === "chat" ? (
        <div className="min-w-0 flex-1 flex flex-col bg-white">
          {selectedChat ? (
            <>
              {/* Chat Header */}
              <div className="h-16 px-6 border-b border-slate-200 bg-white flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="h-10 w-10 rounded-xl bg-slate-100 border border-slate-200 flex items-center justify-center">
                    <UserIcon className="h-5 w-5 text-blue-600" />
                  </div>
                  <div>
                    <h3 className="text-sm font-bold text-slate-900">{selectedChat.username}</h3>
                    <p className="text-xs text-slate-500">{selectedChat.email}</p>
                  </div>
                </div>

                {/* Instant Video Meet Button */}
                <button
                  onClick={async () => {
                    try {
                      const res = await fetch("/api/sessions/create", {
                        method: "POST",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({
                          ownerId: currentUser._id,
                          name: `Call with ${selectedChat.username}`,
                        }),
                      });
                      const data = await res.json();
                      if (!res.ok || !data.sessionId)
                        throw new Error(data.msg || data.message || "Could not create meeting");
                      handleEnterMeeting(data.sessionId, generateSessionKey());
                    } catch (error) {
                      setErrorMessage(
                        error instanceof Error ? error.message : "Could not create meeting",
                      );
                    }
                  }}
                  className="flex items-center gap-2 px-3 py-2 rounded-xl bg-blue-50 hover:bg-blue-50 border border-blue-200 text-blue-600 text-xs font-semibold transition-all"
                  title="Start instant video call"
                >
                  <Video className="w-4 h-4" />
                  <span>Start Video Meet</span>
                </button>
              </div>

              {/* Chat Messages */}
              <div className="flex-1 overflow-y-auto p-6 space-y-4">
                {messages.map((msg, index) => (
                  <div
                    key={index}
                    className={`flex ${msg.fromSelf ? "justify-end" : "justify-start"}`}
                  >
                    <div
                      className={`max-w-md px-4 py-2.5 rounded-2xl text-sm ${
                        msg.fromSelf
                          ? "bg-blue-600 text-white font-medium rounded-br-xs"
                          : "bg-white border border-slate-200 text-slate-700 rounded-bl-xs"
                      }`}
                    >
                      {msg.message}
                    </div>
                  </div>
                ))}
                <div ref={messagesEndRef} />
              </div>

              {/* Chat Input */}
              <form
                onSubmit={handleSendDirectMessage}
                className="p-4 border-t border-slate-200 bg-white"
              >
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    className="p-2.5 text-slate-500 hover:text-slate-900 rounded-xl hover:bg-slate-100 transition-colors"
                  >
                    <Smile className="h-5 w-5" />
                  </button>
                  <input
                    type="text"
                    value={textInput}
                    onChange={(e) => setTextInput(e.target.value)}
                    placeholder={`Message ${selectedChat.username}...`}
                    className="flex-1 rounded-xl bg-white border border-slate-200 px-4 py-2.5 text-sm text-slate-900 placeholder-slate-500 focus:outline-none focus:border-blue-200"
                  />
                  <button
                    type="submit"
                    disabled={!textInput.trim()}
                    className="p-2.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-40 disabled:hover:bg-blue-600 text-white rounded-xl transition-all"
                  >
                    <Send className="h-4 w-4" />
                  </button>
                </div>
              </form>
            </>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-slate-500">
              <div className="p-4 bg-white rounded-2xl border border-slate-200 mb-4">
                <MessageSquare className="h-10 w-10 text-slate-600" />
              </div>
              <h3 className="text-lg font-bold text-slate-700">No Chat Selected</h3>
              <p className="text-sm max-w-sm mt-1">
                Select a user from the sidebar to chat directly, or switch to the Meetings tab.
              </p>
            </div>
          )}
        </div>
      ) : (
        <Lobby currentUser={currentUser} onEnterMeeting={handleEnterMeeting} />
      )}
    </div>
  );
};

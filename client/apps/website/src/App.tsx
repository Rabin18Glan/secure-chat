import React, { useState, useEffect, useRef } from "react";
import { Send, LogOut, MessageSquare, Shield, Smile, User as UserIcon } from "lucide-react";

interface User {
  _id: string;
  username: string;
  email: string;
  avatarImage: string;
}

interface Message {
  fromSelf: boolean;
  message: string;
}

export const App: React.FC = () => {
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [contacts, setContacts] = useState<User[]>([]);
  const [selectedChat, setSelectedChat] = useState<User | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [textInput, setTextInput] = useState("");
  const [isLoginMode, setIsLoginMode] = useState(true);
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [errorMessage, setErrorMessage] = useState("");

  const socketRef = useRef<WebSocket | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const saved = localStorage.getItem("chat-app-user");
    if (saved) {
      try {
        const u = JSON.parse(saved);
        setCurrentUser(u);
      } catch {
        localStorage.removeItem("chat-app-user");
      }
    }
  }, []);

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

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host;
    const ws = new WebSocket(`${protocol}//${host}/ws?userId=${currentUser._id}`);

    ws.onopen = () => {
      ws.send(JSON.stringify({ event: "add-user", userId: currentUser._id }));
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.event === "msg-recieve") {
          setMessages((prev) => [...prev, { fromSelf: false, message: data.msg }]);
        }
      } catch (err) {
        console.error("WS error:", err);
      }
    };

    socketRef.current = ws;

    return () => {
      ws.close();
    };
  }, [currentUser]);

  useEffect(() => {
    if (!currentUser || !selectedChat) return;

    fetch("/api/messages/getmsg", {
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
      .catch((err) => console.error("Error fetching messages:", err));
  }, [currentUser, selectedChat]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

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
        setErrorMessage(data.msg || "Authentication failed");
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
    setSelectedChat(null);
    setMessages([]);
    socketRef.current?.close();
  };

  const handleSendMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!textInput.trim() || !currentUser || !selectedChat) return;

    const msg = textInput.trim();
    setTextInput("");

    if (socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
      socketRef.current.send(
        JSON.stringify({
          event: "send-msg",
          to: selectedChat._id,
          msg,
        }),
      );
    }

    setMessages((prev) => [...prev, { fromSelf: true, message: msg }]);

    await fetch("/api/messages/addmsg", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        from: currentUser._id,
        to: selectedChat._id,
        message: msg,
      }),
    }).catch((err) => console.error("Error saving message:", err));
  };

  if (!currentUser) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-slate-950 p-4 text-slate-100">
        <div className="w-full max-w-md rounded-2xl bg-slate-900 border border-slate-800 p-8 shadow-2xl shadow-cyan-950/20">
          <div className="flex items-center justify-center gap-2 mb-6">
            <div className="p-3 bg-cyan-500/10 rounded-xl border border-cyan-500/30">
              <Shield className="h-8 w-8 text-cyan-400" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight text-white">Secure Chat</h1>
              <p className="text-xs text-slate-400">Axum + Vite+ Toolchain</p>
            </div>
          </div>

          <div className="flex rounded-lg bg-slate-950 p-1 mb-6 border border-slate-800">
            <button
              onClick={() => setIsLoginMode(true)}
              className={`flex-1 py-1.5 text-sm font-medium rounded-md transition-all ${
                isLoginMode
                  ? "bg-cyan-500 text-slate-950 font-semibold"
                  : "text-slate-400 hover:text-white"
              }`}
            >
              Sign In
            </button>
            <button
              onClick={() => setIsLoginMode(false)}
              className={`flex-1 py-1.5 text-sm font-medium rounded-md transition-all ${
                !isLoginMode
                  ? "bg-cyan-500 text-slate-950 font-semibold"
                  : "text-slate-400 hover:text-white"
              }`}
            >
              Register
            </button>
          </div>

          {errorMessage && (
            <div className="mb-4 rounded-lg bg-rose-500/10 border border-rose-500/30 p-3 text-sm text-rose-400">
              {errorMessage}
            </div>
          )}

          <form onSubmit={handleAuth} className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-slate-400 mb-1">Username</label>
              <input
                type="text"
                required
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="johndoe"
                className="w-full rounded-lg bg-slate-950 border border-slate-800 px-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500"
              />
            </div>

            {!isLoginMode && (
              <div>
                <label className="block text-xs font-semibold text-slate-400 mb-1">Email</label>
                <input
                  type="email"
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="john@example.com"
                  className="w-full rounded-lg bg-slate-950 border border-slate-800 px-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500"
                />
              </div>
            )}

            <div>
              <label className="block text-xs font-semibold text-slate-400 mb-1">Password</label>
              <input
                type="password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••••••"
                className="w-full rounded-lg bg-slate-950 border border-slate-800 px-4 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500"
              />
            </div>

            <button
              type="submit"
              className="w-full mt-2 rounded-lg bg-cyan-500 hover:bg-cyan-400 py-2.5 text-sm font-semibold text-slate-950 transition-colors"
            >
              {isLoginMode ? "Sign In" : "Create Account"}
            </button>
          </form>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-slate-950 text-slate-100 overflow-hidden">
      {/* Sidebar */}
      <div className="w-80 border-r border-slate-800 bg-slate-900/70 flex flex-col">
        <div className="p-4 border-b border-slate-800 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-cyan-500/10 rounded-lg border border-cyan-500/30">
              <Shield className="h-5 w-5 text-cyan-400" />
            </div>
            <div>
              <h2 className="text-sm font-bold text-white">{currentUser.username}</h2>
              <p className="text-xs text-emerald-400 flex items-center gap-1">
                <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>{" "}
                Online
              </p>
            </div>
          </div>
          <button
            onClick={handleLogout}
            title="Log Out"
            className="p-2 text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition-colors"
          >
            <LogOut className="h-4 w-4" />
          </button>
        </div>

        <div className="p-3">
          <p className="text-xs font-bold uppercase tracking-wider text-slate-500 px-2 py-1">
            Conversations
          </p>
        </div>

        <div className="flex-1 overflow-y-auto px-2 space-y-1">
          {contacts.map((contact) => {
            const isSelected = selectedChat?._id === contact._id;
            return (
              <button
                key={contact._id}
                onClick={() => setSelectedChat(contact)}
                className={`w-full flex items-center gap-3 p-3 rounded-xl transition-all ${
                  isSelected
                    ? "bg-cyan-500/10 border border-cyan-500/30 text-white"
                    : "hover:bg-slate-800/60 text-slate-300"
                }`}
              >
                <div className="h-10 w-10 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center shrink-0">
                  <UserIcon className="h-5 w-5 text-slate-400" />
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
      </div>

      {/* Chat Area */}
      <div className="flex-1 flex flex-col bg-slate-950">
        {selectedChat ? (
          <>
            <div className="h-16 px-6 border-b border-slate-800 bg-slate-900/40 flex items-center gap-3">
              <div className="h-9 w-9 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center">
                <UserIcon className="h-5 w-5 text-cyan-400" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-white">{selectedChat.username}</h3>
                <p className="text-xs text-slate-400">{selectedChat.email}</p>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-6 space-y-4">
              {messages.map((msg, index) => (
                <div
                  key={index}
                  className={`flex ${msg.fromSelf ? "justify-end" : "justify-start"}`}
                >
                  <div
                    className={`max-w-md px-4 py-2.5 rounded-2xl text-sm ${
                      msg.fromSelf
                        ? "bg-cyan-500 text-slate-950 font-medium rounded-br-xs"
                        : "bg-slate-900 border border-slate-800 text-slate-100 rounded-bl-xs"
                    }`}
                  >
                    {msg.message}
                  </div>
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>

            <form
              onSubmit={handleSendMessage}
              className="p-4 border-t border-slate-800 bg-slate-900/50"
            >
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="p-2 text-slate-400 hover:text-white rounded-lg hover:bg-slate-800 transition-colors"
                >
                  <Smile className="h-5 w-5" />
                </button>
                <input
                  type="text"
                  value={textInput}
                  onChange={(e) => setTextInput(e.target.value)}
                  placeholder={`Message ${selectedChat.username}...`}
                  className="flex-1 rounded-xl bg-slate-950 border border-slate-800 px-4 py-2.5 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500"
                />
                <button
                  type="submit"
                  disabled={!textInput.trim()}
                  className="p-2.5 bg-cyan-500 hover:bg-cyan-400 disabled:opacity-40 disabled:hover:bg-cyan-500 text-slate-950 rounded-xl transition-all"
                >
                  <Send className="h-4 w-4" />
                </button>
              </div>
            </form>
          </>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-slate-500">
            <div className="p-4 bg-slate-900 rounded-2xl border border-slate-800 mb-4">
              <MessageSquare className="h-10 w-10 text-slate-600" />
            </div>
            <h3 className="text-lg font-bold text-slate-300">No Chat Selected</h3>
            <p className="text-sm max-w-sm mt-1">
              Select a user from the sidebar to start a real-time conversation.
            </p>
          </div>
        )}
      </div>
    </div>
  );
};

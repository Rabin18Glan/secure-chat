import React, { useState, useEffect, useRef, useCallback } from "react";
import {
  Mic,
  MicOff,
  Video,
  VideoOff,
  MonitorUp,
  PhoneOff,
  MessageSquare,
  Users,
  Shield,
  Crown,
  Copy,
  Check,
  Send,
  UserCheck,
  UserX,
  ShieldPlus,
  ShieldMinus,
  Lock,
  X,
  AlertTriangle,
} from "lucide-react";
import { encryptMessage, decryptMessage } from "../utils/crypto";

interface User {
  _id: string;
  username: string;
  email: string;
  avatarImage: string;
}

interface Participant {
  userId: string;
  username: string;
  avatar: string;
  role: string;
}

interface WaitingUser {
  userId: string;
  username: string;
  avatar: string;
}

interface ChatMessage {
  id: string;
  senderId: string;
  senderName: string;
  text: string;
  timestamp: string;
  isSelf: boolean;
}

interface MeetingRoomProps {
  currentUser: User;
  sessionId: string;
  sessionKey: string;
  onLeave: () => void;
}

const ICE_SERVERS: RTCConfiguration = {
  iceServers: [{ urls: "stun:stun.l.google.com:19302" }, { urls: "stun:stun1.l.google.com:19302" }],
};

export const MeetingRoom: React.FC<MeetingRoomProps> = ({
  currentUser,
  sessionId,
  sessionKey,
  onLeave,
}) => {
  // Session lifecycle status
  const [sessionStatus, setSessionStatus] = useState<
    "connecting" | "waiting" | "admitted" | "denied" | "kicked" | "error"
  >("connecting");
  const [statusMessage, setStatusMessage] = useState<string>("Connecting to session...");

  // Session state
  const [role, setRole] = useState<string>("member");
  const [ownerId, setOwnerId] = useState<string>("");
  const [admins, setAdmins] = useState<string[]>([]);
  const [participants, setParticipants] = useState<Participant[]>([]);
  const [waitingRoom, setWaitingRoom] = useState<WaitingUser[]>([]);

  // Media Controls
  const [isMicOn, setIsMicOn] = useState<boolean>(true);
  const [isVideoOn, setIsVideoOn] = useState<boolean>(true);
  const [isScreenSharing, setIsScreenSharing] = useState<boolean>(false);
  const [peerMediaState, setPeerMediaState] = useState<
    Record<string, { audio?: boolean; video?: boolean; screen?: boolean }>
  >({});

  // Leave / Terminate confirmation dialog
  const [showLeaveModal, setShowLeaveModal] = useState<boolean>(false);

  // Streams
  const localStreamRef = useRef<MediaStream | null>(null);
  const screenStreamRef = useRef<MediaStream | null>(null);
  const [remoteStreams, setRemoteStreams] = useState<Map<string, MediaStream>>(new Map());

  // WebRTC Peer Connections: targetUserId -> RTCPeerConnection
  const peersRef = useRef<Map<string, RTCPeerConnection>>(new Map());
  const pendingCandidatesRef = useRef<Map<string, RTCIceCandidateInit[]>>(new Map());

  // WebSocket
  const wsRef = useRef<WebSocket | null>(null);

  // Chat
  const [isChatOpen, setIsChatOpen] = useState<boolean>(false);
  const [unreadChatCount, setUnreadChatCount] = useState<number>(0);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [chatInput, setChatInput] = useState<string>("");
  const chatBottomRef = useRef<HTMLDivElement | null>(null);

  // UI Panels
  const [isParticipantsOpen, setIsParticipantsOpen] = useState<boolean>(false);
  const [copiedLink, setCopiedLink] = useState<boolean>(false);

  // Video element refs
  const localVideoRef = useRef<HTMLVideoElement | null>(null);
  const isChatOpenRef = useRef(isChatOpen);
  useEffect(() => {
    isChatOpenRef.current = isChatOpen;
  }, [isChatOpen]);

  useEffect(() => {
    if (localVideoRef.current && sessionStatus === "admitted") {
      localVideoRef.current.srcObject = screenStreamRef.current || localStreamRef.current;
    }
  }, [sessionStatus]);

  const isOwner = role === "owner" || currentUser._id === ownerId;
  const isAdmin = isOwner || role === "admin" || admins.includes(currentUser._id);

  // Setup Local Media Stream
  const initLocalMedia = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: true,
        video: { width: { ideal: 1280 }, height: { ideal: 720 } },
      });
      localStreamRef.current = stream;
      setIsMicOn(stream.getAudioTracks().length > 0);
      setIsVideoOn(stream.getVideoTracks().length > 0);
      if (localVideoRef.current) {
        localVideoRef.current.srcObject = stream;
      }
      return stream;
    } catch (err) {
      console.warn("Could not access camera/mic, falling back to audio only or blank:", err);
      try {
        const audioStream = await navigator.mediaDevices.getUserMedia({ audio: true });
        localStreamRef.current = audioStream;
        setIsVideoOn(false);
        setIsMicOn(true);
        return audioStream;
      } catch {
        console.warn("Could not access audio either.");
        setIsMicOn(false);
        setIsVideoOn(false);
        return null;
      }
    }
  };

  // Helper to send WS JSON
  const sendWs = useCallback((payload: Record<string, any>) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(payload));
    }
  }, []);

  // WebRTC Peer Management
  const createPeer = useCallback(
    (targetUserId: string, initiator: boolean) => {
      if (peersRef.current.has(targetUserId)) {
        return peersRef.current.get(targetUserId)!;
      }

      const pc = new RTCPeerConnection(ICE_SERVERS);
      peersRef.current.set(targetUserId, pc);

      // Add local tracks to peer
      const stream = localStreamRef.current;
      if (stream) {
        stream.getAudioTracks().forEach((track) => pc.addTrack(track, stream));
      }
      const videoTrack =
        screenStreamRef.current?.getVideoTracks()[0] || stream?.getVideoTracks()[0];
      if (videoTrack) {
        pc.addTrack(videoTrack, screenStreamRef.current || stream!);
      } else {
        pc.addTransceiver("video", { direction: "sendrecv" });
      }
      if (!stream?.getAudioTracks().length) pc.addTransceiver("audio", { direction: "recvonly" });

      pc.onicecandidate = (event) => {
        if (event.candidate) {
          sendWs({
            event: "webrtc-ice-candidate",
            sessionId,
            to: targetUserId,
            candidate: event.candidate.toJSON(),
          });
        }
      };

      pc.ontrack = (event) => {
        const [incomingStream] = event.streams;
        if (incomingStream) {
          setRemoteStreams((prev) => {
            const next = new Map(prev);
            next.set(targetUserId, incomingStream);
            return next;
          });
        }
      };

      pc.onconnectionstatechange = () => {
        if (
          pc.connectionState === "disconnected" ||
          pc.connectionState === "failed" ||
          pc.connectionState === "closed"
        ) {
          setRemoteStreams((prev) => {
            const next = new Map(prev);
            next.delete(targetUserId);
            return next;
          });
          peersRef.current.delete(targetUserId);
        }
      };

      if (initiator) {
        pc.onnegotiationneeded = async () => {
          try {
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);
            sendWs({
              event: "webrtc-offer",
              sessionId,
              to: targetUserId,
              sdp: offer,
            });
          } catch (e) {
            console.error("Negotiation error:", e);
          }
        };
      }

      return pc;
    },
    [sendWs, sessionId],
  );

  // Initialize WebSocket Connection & Session Knock
  useEffect(() => {
    let isMounted = true;
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/ws?userId=${currentUser._id}`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = async () => {
      if (!isMounted) return;
      const stream = await initLocalMedia();
      if (!isMounted || ws.readyState !== WebSocket.OPEN) {
        stream?.getTracks().forEach((track) => track.stop());
        return;
      }

      // Register presence and knock for the session
      ws.send(
        JSON.stringify({
          event: "session-knock",
          sessionId,
          userId: currentUser._id,
          username: currentUser.username,
          avatar: currentUser.avatarImage || "",
        }),
      );
    };

    ws.onmessage = async (e) => {
      if (!isMounted) return;
      try {
        const msg = JSON.parse(e.data);

        switch (msg.event) {
          case "session-waiting":
            setSessionStatus("waiting");
            setStatusMessage(msg.msg || "Waiting for host to admit you...");
            break;

          case "session-admitted":
            setSessionStatus("admitted");
            setRole(msg.role);
            setOwnerId(msg.ownerId);
            setAdmins(msg.admins || []);
            setParticipants(
              (msg.participants || []).filter((p: Participant) => p.userId !== currentUser._id),
            );
            setWaitingRoom(msg.waitingRoom || []);

            // Establish peer connections with admitted participants
            (msg.participants || []).forEach((p: Participant) => {
              if (p.userId !== currentUser._id) {
                // Deterministic initiator: lower userId initiates
                const shouldInitiate = currentUser._id < p.userId;
                createPeer(p.userId, shouldInitiate);
              }
            });
            break;

          case "session-denied":
            setSessionStatus("denied");
            setStatusMessage(msg.msg || "Your join request was denied by the host.");
            break;

          case "session-kicked":
            setSessionStatus("kicked");
            setStatusMessage(msg.msg || "You were removed from the session.");
            break;

          case "session-terminated":
            setSessionStatus("kicked");
            setStatusMessage(msg.msg || "The call was ended for everyone by the session admin.");
            break;

          case "session-knock-request":
            // Admin receives notification of knocking user
            setWaitingRoom((prev) => {
              if (prev.some((u) => u.userId === msg.user.userId)) return prev;
              return [...prev, msg.user];
            });
            break;

          case "session-waiting-list":
            setWaitingRoom(msg.waitingRoom || []);
            break;

          case "session-user-joined":
            setParticipants((prev) => {
              if (prev.some((p) => p.userId === msg.user.userId)) return prev;
              return [...prev, msg.user];
            });
            // Connect peer
            if (msg.user.userId !== currentUser._id) {
              const shouldInitiate = currentUser._id < msg.user.userId;
              createPeer(msg.user.userId, shouldInitiate);
            }
            break;

          case "session-user-left":
            setParticipants((prev) => prev.filter((p) => p.userId !== msg.userId));
            setRemoteStreams((prev) => {
              const next = new Map(prev);
              next.delete(msg.userId);
              return next;
            });
            if (peersRef.current.has(msg.userId)) {
              peersRef.current.get(msg.userId)?.close();
              peersRef.current.delete(msg.userId);
            }
            break;

          case "session-roles-updated":
            setAdmins(msg.admins || []);
            setOwnerId(msg.ownerId);
            if (msg.ownerId === currentUser._id) {
              setRole("owner");
            } else if ((msg.admins || []).includes(currentUser._id)) {
              setRole("admin");
            } else {
              setRole("member");
            }
            break;

          case "session-chat-message":
            // Decrypt incoming message with local sessionKey
            try {
              const decrypted = await decryptMessage(msg.ciphertext, msg.iv, sessionKey);
              setChatMessages((prev) => [
                ...prev,
                {
                  id: `${msg.senderId}-${Date.now()}-${Math.random()}`,
                  senderId: msg.senderId,
                  senderName: msg.senderName,
                  text: decrypted,
                  timestamp: msg.timestamp || new Date().toISOString(),
                  isSelf: msg.senderId === currentUser._id,
                },
              ]);
              if (!isChatOpenRef.current && msg.senderId !== currentUser._id) {
                setUnreadChatCount((c) => c + 1);
              }
            } catch (err) {
              console.error("Decryption error:", err);
            }
            break;

          case "webrtc-offer": {
            const senderId = msg.from;
            let pc = peersRef.current.get(senderId);
            if (!pc) {
              pc = createPeer(senderId, false);
            }
            await pc.setRemoteDescription(new RTCSessionDescription(msg.sdp));
            for (const candidate of pendingCandidatesRef.current.get(senderId) || []) {
              await pc.addIceCandidate(new RTCIceCandidate(candidate));
            }
            pendingCandidatesRef.current.delete(senderId);
            const answer = await pc.createAnswer();
            await pc.setLocalDescription(answer);
            sendWs({
              event: "webrtc-answer",
              sessionId,
              to: senderId,
              sdp: answer,
            });
            break;
          }

          case "webrtc-answer": {
            const senderId = msg.from;
            const pc = peersRef.current.get(senderId);
            if (pc) {
              await pc.setRemoteDescription(new RTCSessionDescription(msg.sdp));
              for (const candidate of pendingCandidatesRef.current.get(senderId) || []) {
                await pc.addIceCandidate(new RTCIceCandidate(candidate));
              }
              pendingCandidatesRef.current.delete(senderId);
            }
            break;
          }

          case "webrtc-ice-candidate": {
            const senderId = msg.from;
            const pc = peersRef.current.get(senderId);
            if (pc && pc.remoteDescription) {
              await pc.addIceCandidate(new RTCIceCandidate(msg.candidate)).catch(() => {});
            } else {
              const list = pendingCandidatesRef.current.get(senderId) || [];
              list.push(msg.candidate);
              pendingCandidatesRef.current.set(senderId, list);
            }
            break;
          }

          case "webrtc-media-toggle": {
            if (msg.from && msg.data) {
              setPeerMediaState((prev) => ({
                ...prev,
                [msg.from]: msg.data,
              }));
            }
            break;
          }

          case "session-error":
            setSessionStatus("error");
            setStatusMessage(msg.msg || "Failed to enter session");
            break;
        }
      } catch (err) {
        console.error("WS Parse error in session:", err);
      }
    };

    ws.onclose = () => {
      if (isMounted) {
        setSessionStatus((status) =>
          ["denied", "kicked", "error"].includes(status) ? status : "error",
        );
        setStatusMessage("Connection closed. Leave and rejoin the meeting to reconnect.");
      }
    };
    ws.onerror = () => {
      if (isMounted) setStatusMessage("Unable to connect to the meeting server.");
    };

    return () => {
      isMounted = false;
      // Close media tracks
      localStreamRef.current?.getTracks().forEach((t) => t.stop());
      screenStreamRef.current?.getTracks().forEach((t) => t.stop());
      // Close all peers
      peersRef.current.forEach((pc) => pc.close());
      peersRef.current.clear();
      // Close WS
      ws.onclose = null;
      ws.close();
    };
  }, [
    currentUser._id,
    currentUser.username,
    currentUser.avatarImage,
    sessionId,
    sessionKey,
    createPeer,
    sendWs,
  ]);

  useEffect(() => {
    if (["denied", "kicked", "error"].includes(sessionStatus)) {
      localStreamRef.current?.getTracks().forEach((track) => track.stop());
      screenStreamRef.current?.getTracks().forEach((track) => track.stop());
      peersRef.current.forEach((peer) => peer.close());
      peersRef.current.clear();
    }
  }, [sessionStatus]);

  // Scroll chat to bottom
  useEffect(() => {
    chatBottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatMessages, isChatOpen]);

  // Handle Local Mic Toggle
  const toggleMic = () => {
    if (!localStreamRef.current) return;
    const audioTrack = localStreamRef.current.getAudioTracks()[0];
    if (audioTrack) {
      audioTrack.enabled = !audioTrack.enabled;
      setIsMicOn(audioTrack.enabled);
      sendWs({
        event: "webrtc-media-toggle",
        sessionId,
        data: { audio: audioTrack.enabled, video: isVideoOn, screen: isScreenSharing },
      });
    }
  };

  // Handle Local Video Toggle
  const toggleVideo = () => {
    if (!localStreamRef.current) return;
    const videoTrack = localStreamRef.current.getVideoTracks()[0];
    if (videoTrack) {
      videoTrack.enabled = !videoTrack.enabled;
      setIsVideoOn(videoTrack.enabled);
      sendWs({
        event: "webrtc-media-toggle",
        sessionId,
        data: { audio: isMicOn, video: videoTrack.enabled, screen: isScreenSharing },
      });
    }
  };

  const stopScreenShare = async () => {
    const screen = screenStreamRef.current;
    screenStreamRef.current = null;
    screen?.getTracks().forEach((track) => {
      track.onended = null;
      track.stop();
    });
    const camera = localStreamRef.current?.getVideoTracks()[0] || null;
    await Promise.all(
      Array.from(peersRef.current.values()).map(async (pc) => {
        const sender = pc.getTransceivers().find((t) => t.receiver.track.kind === "video")?.sender;
        if (sender) await sender.replaceTrack(camera);
      }),
    );
    if (localVideoRef.current) localVideoRef.current.srcObject = localStreamRef.current;
    setIsScreenSharing(false);
    sendWs({
      event: "webrtc-media-toggle",
      sessionId,
      data: {
        audio: !!localStreamRef.current?.getAudioTracks().some((t) => t.enabled),
        video: !!camera?.enabled,
        screen: false,
      },
    });
  };

  const toggleScreenShare = async () => {
    if (screenStreamRef.current) {
      await stopScreenShare();
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getDisplayMedia({ video: true });
      const track = stream.getVideoTracks()[0];
      screenStreamRef.current = stream;
      await Promise.all(
        Array.from(peersRef.current.values()).map(async (pc) => {
          const sender = pc
            .getTransceivers()
            .find((t) => t.receiver.track.kind === "video")?.sender;
          if (sender) await sender.replaceTrack(track);
        }),
      );
      if (localVideoRef.current) localVideoRef.current.srcObject = stream;
      setIsScreenSharing(true);
      track.onended = () => {
        void stopScreenShare();
      };
      sendWs({
        event: "webrtc-media-toggle",
        sessionId,
        data: { audio: isMicOn, video: isVideoOn, screen: true },
      });
    } catch (error) {
      console.warn("Screen sharing failed or was cancelled:", error);
      if (screenStreamRef.current) await stopScreenShare();
    }
  };

  // Admin Actions
  const handleApproveUser = (targetUserId: string) => {
    sendWs({
      event: "session-approve-user",
      sessionId,
      targetUserId,
    });
    setWaitingRoom((prev) => prev.filter((u) => u.userId !== targetUserId));
  };

  const handleRejectUser = (targetUserId: string) => {
    sendWs({
      event: "session-reject-user",
      sessionId,
      targetUserId,
    });
    setWaitingRoom((prev) => prev.filter((u) => u.userId !== targetUserId));
  };

  const handlePromoteAdmin = (targetUserId: string) => {
    sendWs({
      event: "session-promote-admin",
      sessionId,
      targetUserId,
    });
  };

  const handleDemoteAdmin = (targetUserId: string) => {
    sendWs({
      event: "session-demote-admin",
      sessionId,
      targetUserId,
    });
  };

  const handleRemoveUser = (targetUserId: string) => {
    sendWs({
      event: "session-remove-user",
      sessionId,
      targetUserId,
    });
  };

  // Leave & Terminate actions
  const handleLeaveCall = () => {
    sendWs({
      event: "session-leave",
      sessionId,
    });
    onLeave();
  };

  const handleEndCallForAll = () => {
    sendWs({
      event: "session-terminate",
      sessionId,
    });
    onLeave();
  };

  // Encrypted Chat Send
  const handleSendChatMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!chatInput.trim()) return;
    const textToSend = chatInput.trim();
    setChatInput("");

    try {
      const { ciphertext, iv } = await encryptMessage(textToSend, sessionKey);
      sendWs({
        event: "session-chat-message",
        sessionId,
        ciphertext,
        iv,
      });
    } catch (err) {
      console.error("Encryption error before sending:", err);
    }
  };

  // Copy Invite Link
  const copyInviteLink = () => {
    const inviteUrl = `${window.location.origin}/?session=${sessionId}#${sessionKey}`;
    void navigator.clipboard.writeText(inviteUrl).catch(() => setCopiedLink(false));
    setCopiedLink(true);
    setTimeout(() => setCopiedLink(false), 2500);
  };

  /* ---------------- Render Waiting / Rejected Screens ---------------- */
  if (sessionStatus === "waiting") {
    return (
      <div className="flex h-screen items-center justify-center bg-white p-4">
        <div className="max-w-md w-full bg-white border border-slate-200 rounded-2xl p-8 text-center space-y-6 ">
          <div className="relative mx-auto w-20 h-20 flex items-center justify-center">
            <div className="absolute inset-0 rounded-full bg-blue-50 animate-ping"></div>
            <div className="relative p-5 bg-blue-50 border border-blue-200 rounded-full">
              <Lock className="w-8 h-8 text-blue-600" />
            </div>
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-bold text-slate-900">Knocking on Session</h2>
            <p className="text-sm text-slate-500">{statusMessage}</p>
            <div className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-slate-100 border border-slate-200 text-xs font-mono text-blue-600 mt-2">
              <span className="w-2 h-2 rounded-full bg-blue-700 animate-pulse"></span>
              {sessionId}
            </div>
          </div>
          <p className="text-xs text-slate-500">
            A session admin or host will review your join request shortly.
          </p>
          <button
            onClick={handleLeaveCall}
            className="w-full py-2.5 px-4 rounded-xl bg-slate-100 hover:bg-slate-100 text-sm font-semibold text-slate-700 transition-colors"
          >
            Cancel & Leave
          </button>
        </div>
      </div>
    );
  }

  if (sessionStatus === "denied" || sessionStatus === "kicked" || sessionStatus === "error") {
    const isEnded =
      statusMessage.toLowerCase().includes("ended") ||
      statusMessage.toLowerCase().includes("terminated");

    return (
      <div className="flex h-screen items-center justify-center bg-white p-4">
        <div className="max-w-md w-full bg-white border border-slate-200 rounded-2xl p-8 text-center space-y-6 ">
          <div className="mx-auto w-16 h-16 rounded-2xl bg-rose-500/10 border border-rose-500/30 flex items-center justify-center text-rose-600">
            <PhoneOff className="w-8 h-8" />
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-bold text-slate-900">
              {isEnded
                ? "Meeting Ended"
                : sessionStatus === "kicked"
                  ? "Removed from Session"
                  : "Access Denied"}
            </h2>
            <p className="text-sm text-slate-500">{statusMessage}</p>
          </div>
          <button
            onClick={onLeave}
            className="w-full py-2.5 px-4 rounded-xl bg-blue-600 hover:bg-blue-700 text-white font-bold text-sm transition-colors  "
          >
            Return to Dashboard
          </button>
        </div>
      </div>
    );
  }

  /* ---------------- Main Meeting Room Screen ---------------- */
  return (
    <div className="relative flex h-screen w-screen bg-white text-slate-700 overflow-hidden select-none">
      {/* Top Header Bar */}
      <header className="absolute top-0 left-0 right-0 h-14 bg-white backdrop-blur-md border-b border-slate-200 px-4 flex items-center justify-between z-20">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 bg-white border border-slate-200 px-3 py-1 rounded-lg">
            <Lock className="w-3.5 h-3.5 text-emerald-700" />
            <span className="text-xs font-mono font-bold text-slate-700">{sessionId}</span>
            <span className="text-[10px] text-emerald-700 font-semibold uppercase tracking-wider bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20">
              E2EE
            </span>
          </div>

          {/* Copy Invite Link */}
          <button
            onClick={copyInviteLink}
            className="flex items-center gap-1.5 text-xs text-slate-700 hover:text-slate-900 bg-slate-100 hover:bg-slate-100 border border-slate-200 px-3 py-1.5 rounded-lg transition-all"
            title="Copy meeting invite link containing end-to-end key"
          >
            {copiedLink ? (
              <Check className="w-3.5 h-3.5 text-emerald-700" />
            ) : (
              <Copy className="w-3.5 h-3.5" />
            )}
            <span>{copiedLink ? "Link Copied!" : "Share Invite"}</span>
          </button>
        </div>

        {/* Admin Knock Alert Badge */}
        {isAdmin && waitingRoom.length > 0 && (
          <button
            onClick={() => setIsParticipantsOpen(true)}
            className="flex items-center gap-2 bg-amber-500/10 border border-amber-500/40 text-amber-700 px-3 py-1 rounded-full text-xs font-semibold animate-pulse hover:bg-amber-500/20 transition-all"
          >
            <Users className="w-3.5 h-3.5" />
            <span>{waitingRoom.length} knocking to enter</span>
          </button>
        )}

        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 text-xs text-slate-500">
            {isOwner ? (
              <span className="inline-flex items-center gap-1 text-amber-700 font-semibold bg-amber-500/10 px-2 py-0.5 rounded border border-amber-500/20">
                <Crown className="w-3 h-3" /> Host (Super Admin)
              </span>
            ) : isAdmin ? (
              <span className="inline-flex items-center gap-1 text-blue-600 font-semibold bg-blue-50 px-2 py-0.5 rounded border border-blue-200">
                <Shield className="w-3 h-3" /> Admin
              </span>
            ) : (
              <span className="inline-flex items-center gap-1 text-slate-500 bg-slate-100 px-2 py-0.5 rounded border border-slate-200">
                Participant
              </span>
            )}
          </div>
        </div>
      </header>

      {/* Main Video Grid Stage */}
      <main className="flex-1 pt-14 pb-20 p-4 flex items-center justify-center overflow-hidden">
        <div
          className={`w-full h-full grid gap-4 place-content-center items-center ${
            remoteStreams.size === 0
              ? "grid-cols-1"
              : remoteStreams.size === 1
                ? "grid-cols-1 md:grid-cols-2"
                : remoteStreams.size <= 3
                  ? "grid-cols-2"
                  : "grid-cols-2 md:grid-cols-3"
          }`}
        >
          {/* Local Video Tile */}
          <div className="relative aspect-video max-h-[82vh] bg-white border border-slate-200 rounded-2xl overflow-hidden  flex items-center justify-center group">
            <video
              ref={localVideoRef}
              autoPlay
              playsInline
              muted
              className={`w-full h-full object-cover transition-opacity ${
                isVideoOn || isScreenSharing ? "opacity-100" : "opacity-0"
              }`}
            />
            {!(isVideoOn || isScreenSharing) && (
              <div className="absolute inset-0 flex flex-col items-center justify-center bg-white">
                <div className="w-20 h-20 rounded-full bg-slate-100 border border-slate-200 flex items-center justify-center text-2xl font-bold text-blue-600">
                  {currentUser.username[0]?.toUpperCase() || "U"}
                </div>
                <p className="mt-3 text-sm text-slate-500">Camera Off</p>
              </div>
            )}

            {/* Local Tile Overlays */}
            <div className="absolute bottom-3 left-3 flex items-center gap-2 bg-white backdrop-blur-sm border border-slate-200 px-3 py-1 rounded-xl text-xs font-semibold">
              <span>{currentUser.username} (You)</span>
              {isOwner && <Crown className="w-3.5 h-3.5 text-amber-700" />}
            </div>

            <div className="absolute bottom-3 right-3 flex items-center gap-1.5">
              {!isMicOn && (
                <div className="p-1.5 rounded-lg bg-rose-500/80 text-slate-900">
                  <MicOff className="w-3.5 h-3.5" />
                </div>
              )}
            </div>
          </div>

          {/* Remote Video Tiles */}
          {Array.from(remoteStreams.entries()).map(([peerId, stream]) => {
            const participant = participants.find((p) => p.userId === peerId);
            const media = peerMediaState[peerId] || {};
            const isPeerVideoActive = media.video !== false;
            const isPeerMicActive = media.audio !== false;

            return (
              <RemoteVideoTile
                key={peerId}
                peerId={peerId}
                stream={stream}
                participant={participant}
                isVideoOn={isPeerVideoActive}
                isMicOn={isPeerMicActive}
                isOwner={peerId === ownerId}
                isAdmin={admins.includes(peerId)}
              />
            );
          })}
        </div>
      </main>

      {/* Floating Bottom Control Bar */}
      <footer className="absolute bottom-0 left-0 right-0 h-20 bg-white backdrop-blur-md border-t border-slate-200 px-6 flex items-center justify-between z-20">
        <div className="hidden sm:flex items-center gap-2 text-xs text-slate-500">
          <span className="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span>{participants.length + 1} in meeting</span>
        </div>

        {/* Media Controls */}
        <div className="flex items-center gap-3 mx-auto">
          {/* Mic */}
          <button
            onClick={toggleMic}
            className={`p-3.5 rounded-2xl transition-all  ${
              isMicOn
                ? "bg-slate-100 hover:bg-slate-100 text-slate-700 border border-slate-200"
                : "bg-rose-500 hover:bg-rose-600 text-white "
            }`}
            title={isMicOn ? "Mute Microphone" : "Unmute Microphone"}
          >
            {isMicOn ? <Mic className="w-5 h-5" /> : <MicOff className="w-5 h-5" />}
          </button>

          {/* Camera */}
          <button
            onClick={toggleVideo}
            className={`p-3.5 rounded-2xl transition-all  ${
              isVideoOn
                ? "bg-slate-100 hover:bg-slate-100 text-slate-700 border border-slate-200"
                : "bg-rose-500 hover:bg-rose-600 text-white "
            }`}
            title={isVideoOn ? "Turn off camera" : "Turn on camera"}
          >
            {isVideoOn ? <Video className="w-5 h-5" /> : <VideoOff className="w-5 h-5" />}
          </button>

          {/* Screen Share */}
          <button
            onClick={toggleScreenShare}
            className={`p-3.5 rounded-2xl transition-all border border-slate-200  ${
              isScreenSharing
                ? "bg-blue-600 text-white font-bold"
                : "bg-slate-100 hover:bg-slate-100 text-slate-700"
            }`}
            title={isScreenSharing ? "Stop sharing screen" : "Share screen"}
          >
            <MonitorUp className="w-5 h-5" />
          </button>

          {/* Leave / End Call */}
          {isAdmin ? (
            <button
              onClick={() => setShowLeaveModal(true)}
              className="p-3.5 rounded-2xl bg-rose-600 hover:bg-rose-500 text-white transition-all   flex items-center gap-1.5"
              title="Leave or End Call for All"
            >
              <PhoneOff className="w-5 h-5" />
            </button>
          ) : (
            <button
              onClick={handleLeaveCall}
              className="p-3.5 rounded-2xl bg-rose-600 hover:bg-rose-500 text-white transition-all  "
              title="Leave Meeting"
            >
              <PhoneOff className="w-5 h-5" />
            </button>
          )}
        </div>

        {/* Drawers Toggle */}
        <div className="flex items-center gap-2">
          {/* Participants & Waiting Room */}
          <button
            onClick={() => setIsParticipantsOpen(!isParticipantsOpen)}
            className={`relative p-3 rounded-xl border transition-all ${
              isParticipantsOpen
                ? "bg-blue-50 border-blue-200 text-blue-600"
                : "bg-slate-100 hover:bg-slate-100 border-slate-200 text-slate-700"
            }`}
            title="Participants"
          >
            <Users className="w-5 h-5" />
            {waitingRoom.length > 0 && isAdmin && (
              <span className="absolute -top-1 -right-1 w-5 h-5 rounded-full bg-amber-500 text-white text-[10px] font-black flex items-center justify-center animate-bounce">
                {waitingRoom.length}
              </span>
            )}
          </button>

          {/* Chat Drawer */}
          <button
            onClick={() => {
              setIsChatOpen(!isChatOpen);
              if (!isChatOpen) setUnreadChatCount(0);
            }}
            className={`relative p-3 rounded-xl border transition-all ${
              isChatOpen
                ? "bg-blue-50 border-blue-200 text-blue-600"
                : "bg-slate-100 hover:bg-slate-100 border-slate-200 text-slate-700"
            }`}
            title="Encrypted Chat"
          >
            <MessageSquare className="w-5 h-5" />
            {unreadChatCount > 0 && !isChatOpen && (
              <span className="absolute -top-1 -right-1 w-5 h-5 rounded-full bg-blue-600 text-white text-[10px] font-black flex items-center justify-center">
                {unreadChatCount}
              </span>
            )}
          </button>
        </div>
      </footer>

      {/* Admin Leave/End Call Modal */}
      {showLeaveModal && (
        <div className="fixed inset-0 bg-white backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-white border border-slate-200 rounded-2xl p-6 max-w-sm w-full space-y-5 ">
            <div className="flex items-center gap-3">
              <div className="p-3 bg-rose-500/10 border border-rose-500/20 rounded-2xl text-rose-600">
                <AlertTriangle className="w-6 h-6" />
              </div>
              <div>
                <h3 className="font-bold text-slate-900 text-base">Leave or End Meeting?</h3>
                <p className="text-xs text-slate-500">You are an admin of this session.</p>
              </div>
            </div>

            <div className="space-y-2 pt-2">
              <button
                onClick={handleEndCallForAll}
                className="w-full py-3 px-4 rounded-xl bg-rose-600 hover:bg-rose-500 text-white font-bold text-xs transition-colors flex items-center justify-center gap-2  "
              >
                <PhoneOff className="w-4 h-4" />
                <span>End Call for Everyone</span>
              </button>

              <button
                onClick={handleLeaveCall}
                className="w-full py-2.5 px-4 rounded-xl bg-slate-100 hover:bg-slate-100 text-slate-700 font-semibold text-xs transition-colors"
              >
                Just Leave Call
              </button>

              <button
                onClick={() => setShowLeaveModal(false)}
                className="w-full py-2 text-xs text-slate-500 hover:text-slate-500 font-medium transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Participants & Waiting Room Drawer */}
      {isParticipantsOpen && (
        <aside className="absolute right-0 top-14 bottom-20 w-80 bg-white border-l border-slate-200 z-30 flex flex-col  animate-in slide-in-from-right-10 duration-200">
          <div className="p-4 border-b border-slate-200 flex items-center justify-between">
            <h3 className="font-bold text-sm text-slate-900 flex items-center gap-2">
              <Users className="w-4 h-4 text-blue-600" />
              Participants ({participants.length + 1})
            </h3>
            <button
              onClick={() => setIsParticipantsOpen(false)}
              className="p-1 rounded-lg hover:bg-slate-100 text-slate-500 hover:text-slate-900"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-4 space-y-6">
            {/* Waiting Room Section for Admins */}
            {isAdmin && (
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <h4 className="text-xs font-bold uppercase tracking-wider text-amber-700 flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse"></span>
                    Waiting Room ({waitingRoom.length})
                  </h4>
                </div>

                {waitingRoom.length === 0 ? (
                  <p className="text-xs text-slate-500 italic">No one waiting to join.</p>
                ) : (
                  <div className="space-y-2">
                    {waitingRoom.map((w) => (
                      <div
                        key={w.userId}
                        className="p-2.5 rounded-xl bg-white border border-amber-500/30 flex items-center justify-between"
                      >
                        <div className="min-w-0 flex-1 mr-2">
                          <p className="text-sm font-semibold truncate text-slate-700">
                            {w.username}
                          </p>
                          <p className="text-[10px] text-amber-700">Knocking...</p>
                        </div>
                        <div className="flex items-center gap-1 shrink-0">
                          <button
                            onClick={() => handleApproveUser(w.userId)}
                            className="p-1.5 rounded-lg bg-emerald-600/30 hover:bg-emerald-600 text-emerald-700 hover:text-slate-900 transition-colors"
                            title="Admit user"
                          >
                            <UserCheck className="w-4 h-4" />
                          </button>
                          <button
                            onClick={() => handleRejectUser(w.userId)}
                            className="p-1.5 rounded-lg bg-rose-600/30 hover:bg-rose-600 text-rose-600 hover:text-white transition-colors"
                            title="Deny request"
                          >
                            <UserX className="w-4 h-4" />
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* In-Meeting Participants */}
            <div className="space-y-3">
              <h4 className="text-xs font-bold uppercase tracking-wider text-slate-500">
                In Meeting
              </h4>

              {/* Self */}
              <div className="p-2.5 rounded-xl bg-white border border-slate-200 flex items-center justify-between">
                <div>
                  <p className="text-sm font-semibold text-slate-700">
                    {currentUser.username} <span className="text-xs text-slate-500">(You)</span>
                  </p>
                  <span className="text-[10px] text-blue-600 font-medium">
                    {isOwner ? "Host / Super Admin" : isAdmin ? "Admin" : "Member"}
                  </span>
                </div>
                {isOwner && <Crown className="w-4 h-4 text-amber-700" />}
              </div>

              {/* Others */}
              {participants.map((p) => {
                const pIsOwner = p.userId === ownerId;
                const pIsAdmin = admins.includes(p.userId);

                return (
                  <div
                    key={p.userId}
                    className="p-2.5 rounded-xl bg-white border border-slate-200 flex items-center justify-between group"
                  >
                    <div>
                      <p className="text-sm font-semibold text-slate-700">{p.username}</p>
                      <span className="text-[10px] text-slate-500 font-medium">
                        {pIsOwner ? "Host / Super Admin" : pIsAdmin ? "Admin" : "Member"}
                      </span>
                    </div>

                    {/* Admin Actions */}
                    {isAdmin && !pIsOwner && (
                      <div className="flex items-center gap-1 opacity-80 group-hover:opacity-100 transition-opacity">
                        {!pIsAdmin ? (
                          <button
                            onClick={() => handlePromoteAdmin(p.userId)}
                            className="p-1.5 rounded-lg hover:bg-blue-50 text-slate-500 hover:text-blue-600 transition-colors"
                            title="Make Admin"
                          >
                            <ShieldPlus className="w-4 h-4" />
                          </button>
                        ) : (
                          <button
                            onClick={() => handleDemoteAdmin(p.userId)}
                            className="p-1.5 rounded-lg hover:bg-amber-500/20 text-slate-500 hover:text-amber-700 transition-colors"
                            title="Remove Admin"
                          >
                            <ShieldMinus className="w-4 h-4" />
                          </button>
                        )}

                        <button
                          onClick={() => handleRemoveUser(p.userId)}
                          className="p-1.5 rounded-lg hover:bg-rose-500/20 text-slate-500 hover:text-rose-600 transition-colors"
                          title="Remove from meeting"
                        >
                          <UserX className="w-4 h-4" />
                        </button>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </aside>
      )}

      {/* Encrypted Chat Drawer */}
      {isChatOpen && (
        <aside className="absolute right-0 top-14 bottom-20 w-80 bg-white border-l border-slate-200 z-30 flex flex-col  animate-in slide-in-from-right-10 duration-200">
          <div className="p-4 border-b border-slate-200 flex items-center justify-between">
            <div>
              <h3 className="font-bold text-sm text-slate-900 flex items-center gap-2">
                <MessageSquare className="w-4 h-4 text-blue-600" />
                In-Meeting Chat
              </h3>
              <p className="text-[10px] text-emerald-700 flex items-center gap-1 mt-0.5">
                <Lock className="w-2.5 h-2.5" /> End-to-End Encrypted (AES-GCM-256)
              </p>
            </div>
            <button
              onClick={() => setIsChatOpen(false)}
              className="p-1 rounded-lg hover:bg-slate-100 text-slate-500 hover:text-slate-900"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Messages */}
          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            {chatMessages.length === 0 ? (
              <div className="h-full flex flex-col items-center justify-center text-center text-slate-500 p-4">
                <Lock className="w-8 h-8 text-slate-600 mb-2" />
                <p className="text-xs">
                  Messages are encrypted client-side with the meeting key from the URL hash.
                </p>
                <p className="text-[10px] text-slate-600 mt-1">
                  Plaintext never touches the server.
                </p>
              </div>
            ) : (
              chatMessages.map((msg) => (
                <div
                  key={msg.id}
                  className={`flex flex-col ${msg.isSelf ? "items-end" : "items-start"}`}
                >
                  <span className="text-[10px] text-slate-500 mb-1 px-1">
                    {msg.isSelf ? "You" : msg.senderName}
                  </span>
                  <div
                    className={`max-w-[85%] px-3 py-2 rounded-2xl text-xs break-words ${
                      msg.isSelf
                        ? "bg-blue-600 text-white font-medium rounded-br-xs"
                        : "bg-white border border-slate-200 text-slate-700 rounded-bl-xs"
                    }`}
                  >
                    {msg.text}
                  </div>
                </div>
              ))
            )}
            <div ref={chatBottomRef} />
          </div>

          {/* Send Input */}
          <form onSubmit={handleSendChatMessage} className="p-3 border-t border-slate-200 bg-white">
            <div className="flex items-center gap-2">
              <input
                type="text"
                placeholder="Send encrypted message..."
                value={chatInput}
                onChange={(e) => setChatInput(e.target.value)}
                className="flex-1 bg-white border border-slate-200 rounded-xl px-3 py-2 text-xs text-slate-900 placeholder-slate-500 focus:outline-none focus:border-blue-200"
              />
              <button
                type="submit"
                disabled={!chatInput.trim()}
                className="p-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-40 text-white rounded-xl transition-all"
              >
                <Send className="w-4 h-4" />
              </button>
            </div>
          </form>
        </aside>
      )}
    </div>
  );
};

// Remote Video Tile Component
interface RemoteVideoTileProps {
  peerId: string;
  stream: MediaStream;
  participant?: Participant;
  isVideoOn: boolean;
  isMicOn: boolean;
  isOwner: boolean;
  isAdmin: boolean;
}

const RemoteVideoTile: React.FC<RemoteVideoTileProps> = ({
  stream,
  participant,
  isVideoOn,
  isMicOn,
  isOwner,
  isAdmin,
}) => {
  const videoRef = useRef<HTMLVideoElement | null>(null);

  useEffect(() => {
    if (videoRef.current) {
      videoRef.current.srcObject = stream;
    }
  }, [stream]);

  const username = participant?.username || "Guest";

  return (
    <div className="relative aspect-video max-h-[82vh] bg-white border border-slate-200 rounded-2xl overflow-hidden  flex items-center justify-center">
      <video
        ref={videoRef}
        autoPlay
        playsInline
        className={`w-full h-full object-cover transition-opacity ${
          isVideoOn ? "opacity-100" : "opacity-0"
        }`}
      />
      {!isVideoOn && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-white">
          <div className="w-20 h-20 rounded-full bg-slate-100 border border-slate-200 flex items-center justify-center text-2xl font-bold text-slate-700">
            {username[0]?.toUpperCase() || "U"}
          </div>
          <p className="mt-3 text-sm text-slate-500">{username}</p>
        </div>
      )}

      {/* Overlay info */}
      <div className="absolute bottom-3 left-3 flex items-center gap-2 bg-white backdrop-blur-sm border border-slate-200 px-3 py-1 rounded-xl text-xs font-semibold">
        <span>{username}</span>
        {isOwner ? (
          <Crown className="w-3.5 h-3.5 text-amber-700" />
        ) : isAdmin ? (
          <Shield className="w-3.5 h-3.5 text-blue-600" />
        ) : null}
      </div>

      <div className="absolute bottom-3 right-3 flex items-center gap-1.5">
        {!isMicOn && (
          <div className="p-1.5 rounded-lg bg-rose-500/80 text-slate-900">
            <MicOff className="w-3.5 h-3.5" />
          </div>
        )}
      </div>
    </div>
  );
};

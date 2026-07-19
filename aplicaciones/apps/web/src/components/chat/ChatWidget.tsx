import { MessageCircle, X } from "lucide-react";
import { useState } from "react";
import { ChatPanel } from "./ChatPanel";

export function ChatWidget() {
  const [open, setOpen] = useState(false);
  return (
    <>
      {open && (
        <div className="fixed bottom-24 right-4 z-40 w-[calc(100vw-2rem)] max-w-md">
          <ChatPanel compact />
        </div>
      )}
      <button
        className="fixed bottom-5 right-5 z-40 grid h-14 w-14 place-items-center rounded-full bg-brand text-white shadow-soft"
        onClick={() => setOpen((value) => !value)}
        title={open ? "Закрыть чат" : "Открыть чат"}
      >
        {open ? <X className="h-6 w-6" /> : <MessageCircle className="h-6 w-6" />}
      </button>
    </>
  );
}

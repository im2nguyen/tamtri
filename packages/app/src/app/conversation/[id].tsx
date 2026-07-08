import { useLocalSearchParams } from "expo-router";

import { ConversationPane } from "@/components/layout/conversation-pane";

export default function ConversationScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  if (!id || Array.isArray(id)) return null;
  return <ConversationPane conversationId={id} />;
}

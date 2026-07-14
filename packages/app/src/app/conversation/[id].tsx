import { Redirect, useLocalSearchParams } from "expo-router";

import { ConversationPane } from "@/components/layout/conversation-pane";
import { isConversationRouteId } from "@/lib/route-path";

export default function ConversationScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  if (!isConversationRouteId(id)) return <Redirect href="/" />;
  return <ConversationPane conversationId={id} />;
}

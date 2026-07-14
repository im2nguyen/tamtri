import { UsageSection as ProviderUsage } from "@/components/health/usage-section";
import { useHarnessUsage } from "@/hooks/use-harness-usage";

export function UsageSection() {
  const { view, refresh } = useHarnessUsage();
  return <ProviderUsage view={view} onRefresh={() => void refresh()} />;
}

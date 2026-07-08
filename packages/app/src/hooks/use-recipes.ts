import { useCallback, useEffect, useState } from "react";
import { method, type RecipeSummary } from "@tamtri/protocol";

import { useDaemon } from "@/runtime/daemon-provider";

export function useRecipes() {
  const { client, serverInfo } = useDaemon();
  const [recipes, setRecipes] = useState<RecipeSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const enabled = Boolean(serverInfo?.features.orchestration);

  const refresh = useCallback(async () => {
    if (!enabled) {
      setRecipes([]);
      return;
    }
    setLoading(true);
    try {
      const rows = await client.request<RecipeSummary[]>(method.RECIPES_LIST);
      setRecipes(rows);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [client, enabled]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const loadRecipeJson = useCallback(
    async (recipeId: string) => {
      const result = await client.request<{ recipe_json: string }>(method.RECIPES_LOAD, {
        recipe_id: recipeId,
      });
      return result.recipe_json;
    },
    [client],
  );

  return { recipes, loading, error, enabled, refresh, loadRecipeJson };
}

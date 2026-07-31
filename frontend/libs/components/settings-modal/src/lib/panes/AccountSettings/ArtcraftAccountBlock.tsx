import { useState, useEffect } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faSpinnerThird } from "@fortawesome/pro-solid-svg-icons";
import { Button } from "@storyteller/ui-button";

import { UsersApi } from "@storyteller/api";
import { UserInfo } from "@storyteller/api";
import { invoke } from "@tauri-apps/api/core";

const usersApi = new UsersApi();

export interface ArtcraftAccountBlockProps {
  globalAccountLogoutCallback: () => void;
}

export const ArtcraftAccountBlock = ({
  globalAccountLogoutCallback,
}: ArtcraftAccountBlockProps) => {
  const [artcraftSession, setArtcraftSession] = useState<UserInfo | undefined>(
    undefined
  );
  const [isLoggedIn, setIsLoggedIn] = useState<boolean>(false);
  const [isCheckingArtcraftSession, setIsCheckingArtcraftSession] =
    useState(false);

  useEffect(() => {
    const fetchSession = async () => {
      setIsCheckingArtcraftSession(true);
      try {
        const result = await usersApi.GetSession();
        console.log(">>> result", result);
        setArtcraftSession(result?.data?.user);
        setIsLoggedIn(result?.data?.loggedIn || false);
      } catch (e) {
        console.error("Error fetching Artcraft session", e);
        setArtcraftSession(undefined);
        setIsLoggedIn(false);
      } finally {
        setIsCheckingArtcraftSession(false);
      }
    };
    fetchSession();
  }, []);

  // NB: Login is handled by the Tauri side now; this block only surfaces the
  // current session and offers a logout.
  const handleLogout = async () => {
    if (isCheckingArtcraftSession || !isLoggedIn) return;
    setIsCheckingArtcraftSession(true);
    await usersApi.Logout();
    setArtcraftSession(undefined);
    setIsLoggedIn(false);
    setIsCheckingArtcraftSession(false);
    globalAccountLogoutCallback(); // TODO: This resets the old global application state

    await invoke("storyteller_purge_credentials_command");
  };

  return (
    <div className="flex justify-between items-center">
      <span>ArtCraft Account:</span>
      <pre>{artcraftSession?.display_name}</pre>
      {isLoggedIn || isCheckingArtcraftSession ? (
        <Button
          variant={isCheckingArtcraftSession ? "secondary" : "destructive"}
          className="h-[30px]"
          onClick={handleLogout}
          disabled={isCheckingArtcraftSession}
        >
          {isCheckingArtcraftSession ? (
            <FontAwesomeIcon
              icon={faSpinnerThird}
              className="animate-spin text-sm"
            />
          ) : (
            "Log Out"
          )}
        </Button>
      ) : (
        <span className="text-white/40">Not logged in</span>
      )}
    </div>
  );
};

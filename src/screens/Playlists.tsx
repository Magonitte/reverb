import { ListMusic } from "lucide-react";
import { useParams } from "react-router";
import { PlaceholderScreen } from "@/components/PlaceholderScreen";

/** `/playlists` e `/playlists/:id` (detalhe; a F11 preenche). */
export default function Playlists() {
  const { id } = useParams();
  return <PlaceholderScreen ns={id ? "playlistDetail" : "playlists"} icon={ListMusic} />;
}

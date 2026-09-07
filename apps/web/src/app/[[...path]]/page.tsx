import { View } from "./view";

export default async function Page({ params }: { params: Promise<{ path?: string[] }> }) {
  const { path } = await params;
  return <View path={path?.join("/") ?? ""}/>;
}

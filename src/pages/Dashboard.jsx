import { useState } from "react";
import { Navigate } from "react-router-dom";

import MainLayout from "../components/layout/MainLayout";
import RaceResultView from "../components/race/RaceResultView";
import EndOfSeasonView from "../components/season/EndOfSeasonView";
import PreSeasonView from "../components/season/PreSeasonView";
import useCareerStore from "../stores/useCareerStore";
import CalendarTab from "./tabs/CalendarTab";
import MyTeamTab from "./tabs/MyTeamTab";
import NextRaceTab from "./tabs/NextRaceTab";
import StandingsTab from "./tabs/StandingsTab";

function Dashboard() {
  const isLoaded = useCareerStore((state) => state.isLoaded);
  const showResult = useCareerStore((state) => state.showResult);
  const lastRaceResult = useCareerStore((state) => state.lastRaceResult);
  const dismissResult = useCareerStore((state) => state.dismissResult);
  const showEndOfSeason = useCareerStore((state) => state.showEndOfSeason);
  const endOfSeasonResult = useCareerStore((state) => state.endOfSeasonResult);
  const showPreseason = useCareerStore((state) => state.showPreseason);
  const [activeTab, setActiveTab] = useState("standings");

  if (!isLoaded) {
    return <Navigate to="/menu" replace />;
  }

  function renderTab() {
    switch (activeTab) {
      case "next-race":
        return <NextRaceTab />;
      case "my-team":
        return <MyTeamTab />;
      case "calendar":
        return <CalendarTab />;
      case "standings":
      default:
        return <StandingsTab />;
    }
  }

  if (showResult && lastRaceResult) {
    return (
      <MainLayout activeTab={activeTab} onTabChange={setActiveTab}>
        <RaceResultView result={lastRaceResult} onDismiss={dismissResult} />
      </MainLayout>
    );
  }

  if (showEndOfSeason && endOfSeasonResult) {
    return (
      <MainLayout activeTab={activeTab} onTabChange={setActiveTab}>
        <EndOfSeasonView />
      </MainLayout>
    );
  }

  if (showPreseason) {
    return (
      <MainLayout activeTab={activeTab} onTabChange={setActiveTab}>
        <PreSeasonView />
      </MainLayout>
    );
  }

  return (
    <MainLayout activeTab={activeTab} onTabChange={setActiveTab}>
      {renderTab()}
    </MainLayout>
  );
}

export default Dashboard;

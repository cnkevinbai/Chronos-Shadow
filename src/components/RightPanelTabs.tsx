// 工作台右侧 Tab 面板 — 三合一容器
// 🛡️ 安全防线 | 🧩 技能中枢 | 🔗 WorkBuddy
//
// 解决：原 3 列独立面板宽度不足，AppGlueBinder 内部三栏完全无法显示
// 方案：Tab 切换 + 共享 w-80 容器，每面板获得完整 320px 渲染空间

import { useState } from "react";
import { useT } from "@/lib/i18n-context";
import { Shield, Zap, Link2 } from "lucide-react";
import RedlineGuardPanel from "@/views/RedlineGuardPanel";
import SecurityShieldPanel from "@/components/SecurityShieldPanel";
import SkillMcpHub from "@/views/SkillMcpHub";
import AppGlueBinder from "@/views/AppGlueBinder";
import type { RedlineStatus } from "@/lib/types";

interface RightPanelTabsProps {
  redlineStatus: RedlineStatus | null;
}

type TabId = "security" | "skills" | "workbuddy";

export default function RightPanelTabs({ redlineStatus }: RightPanelTabsProps) {
  const t = useT();
  const [activeTab, setActiveTab] = useState<TabId>("security");

  const tabs: { id: TabId; icon: React.ComponentType<{ className?: string }>; label: string; badge?: string }[] = [
    { id: "security", icon: Shield, label: t.right_tab_security, badge: t.live_badge },
    { id: "skills", icon: Zap, label: t.right_tab_skills },
    { id: "workbuddy", icon: Link2, label: t.right_tab_workbuddy },
  ];

  return (
    <aside className="w-80 bg-[#0c0c0e] overflow-hidden flex flex-col shrink-0 border-l border-[#27272a]">
      {/* Tab 导航 */}
      <div className="flex border-b border-[#27272a] bg-[#121214] shrink-0">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex-1 flex items-center justify-center space-x-1.5 py-2.5 text-[10px] font-medium transition-all relative ${
                isActive
                  ? "text-white bg-[#0c0c0e]"
                  : "text-zinc-500 hover:text-zinc-300 hover:bg-[#0c0c0e]/50"
              }`}
            >
              <Icon className={`w-3 h-3 ${isActive ? (tab.id === "security" ? "text-emerald-400" : tab.id === "skills" ? "text-amber-400" : "text-cyan-400") : ""}`} />
              <span className="hidden xl:inline">{tab.label}</span>
              {tab.badge && (
                <span className="text-[8px] px-1 py-0.5 rounded bg-emerald-950/50 text-emerald-400 border border-emerald-800/30">
                  {tab.badge}
                </span>
              )}
              {isActive && (
                <div className={`absolute bottom-0 left-2 right-2 h-0.5 rounded-full ${
                  tab.id === "security" ? "bg-emerald-400" : tab.id === "skills" ? "bg-amber-400" : "bg-cyan-400"
                }`} />
              )}
            </button>
          );
        })}
      </div>

      {/* Tab 内容 */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {activeTab === "security" && (
          <div className="flex-1 flex flex-col overflow-hidden">
            <div className="flex-1 overflow-hidden">
              <RedlineGuardPanel redlineStatus={redlineStatus} />
            </div>
            <div className="h-[45%] border-t border-[#27272a] overflow-hidden">
              <SecurityShieldPanel redlineStatus={redlineStatus} />
            </div>
          </div>
        )}
        {activeTab === "skills" && (
          <div className="flex-1 overflow-hidden">
            <SkillMcpHub />
          </div>
        )}
        {activeTab === "workbuddy" && (
          <div className="flex-1 overflow-hidden">
            <AppGlueBinder />
          </div>
        )}
      </div>
    </aside>
  );
}

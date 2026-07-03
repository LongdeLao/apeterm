use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, EnumIter, IntoStaticStr)]
pub enum Key {
    #[strum(serialize = "app.language.english")]
    AppLanguageEnglish,
    #[strum(serialize = "app.language.german")]
    AppLanguageGerman,
    #[strum(serialize = "app.theme.dark")]
    AppThemeDark,
    #[strum(serialize = "app.theme.light")]
    AppThemeLight,
    #[strum(serialize = "app.theme.transparent")]
    AppThemeTransparent,
    #[strum(serialize = "app.theme.bloomberg")]
    AppThemeBloomberg,
    #[strum(serialize = "onboarding.prompt.continue")]
    OnboardingPromptContinue,
    #[strum(serialize = "onboarding.title.language")]
    OnboardingTitleLanguage,
    #[strum(serialize = "onboarding.title.theme")]
    OnboardingTitleTheme,
    #[strum(serialize = "panel.title.news")]
    PanelTitleNews,
    #[strum(serialize = "panel.title.watchlist")]
    PanelTitleWatchlist,
    #[strum(serialize = "panel.title.calendar")]
    PanelTitleCalendar,
    #[strum(serialize = "panel.title.notes")]
    PanelTitleNotes,
    #[strum(serialize = "panel.title.sec")]
    PanelTitleSec,
    #[strum(serialize = "panel.title.agent")]
    PanelTitleAgent,
    #[strum(serialize = "panel.title.picker")]
    PanelTitlePicker,
    #[strum(serialize = "app.footer")]
    AppFooter,
    #[strum(serialize = "dashboard.help.title")]
    DashboardHelpTitle,
    #[strum(serialize = "dashboard.help.title_split")]
    DashboardHelpTitleSplit,
    #[strum(serialize = "dashboard.help.focus_tab")]
    DashboardHelpFocusTab,
    #[strum(serialize = "dashboard.help.focus_np")]
    DashboardHelpFocusNp,
    #[strum(serialize = "dashboard.help.move_focus")]
    DashboardHelpMoveFocus,
    #[strum(serialize = "dashboard.help.resize")]
    DashboardHelpResize,
    #[strum(serialize = "dashboard.help.change_pane")]
    DashboardHelpChangePane,
    #[strum(serialize = "dashboard.help.settings")]
    DashboardHelpSettings,
    #[strum(serialize = "dashboard.help.edit_watchlist")]
    DashboardHelpEditWatchlist,
    #[strum(serialize = "dashboard.help.split")]
    DashboardHelpSplit,
    #[strum(serialize = "dashboard.help.add_panel")]
    DashboardHelpAddPanel,
    #[strum(serialize = "dashboard.help.search")]
    DashboardHelpSearch,
    #[strum(serialize = "dashboard.help.toggle_locale")]
    DashboardHelpToggleLocale,
    #[strum(serialize = "dashboard.help.drag")]
    DashboardHelpDrag,
    #[strum(serialize = "dashboard.help.click")]
    DashboardHelpClick,
    #[strum(serialize = "dashboard.help.close_panel")]
    DashboardHelpClosePanel,
    #[strum(serialize = "dashboard.help.reset")]
    DashboardHelpReset,
    #[strum(serialize = "dashboard.help.close_help")]
    DashboardHelpCloseHelp,
    #[strum(serialize = "dashboard.help.quit")]
    DashboardHelpQuit,
    #[strum(serialize = "watchlist.section.crypto")]
    WatchlistSectionCrypto,
    #[strum(serialize = "watchlist.section.stocks")]
    WatchlistSectionStocks,
    #[strum(serialize = "watchlist.status.connecting_binance")]
    WatchlistStatusConnectingBinance,
    #[strum(serialize = "watchlist.status.live_quotes_pending")]
    WatchlistStatusLiveQuotesPending,
    #[strum(serialize = "watchlist.status.loading_quote")]
    WatchlistStatusLoadingQuote,
    #[strum(serialize = "watchlist.market.pending")]
    WatchlistMarketPending,
    #[strum(serialize = "watchlist.edit.help")]
    WatchlistEditHelp,
    #[strum(serialize = "watchlist.edit.input_footer")]
    WatchlistEditInputFooter,
    #[strum(serialize = "watchlist.footer.edit")]
    WatchlistFooterEdit,
    #[strum(serialize = "watchlist.edit.add_stock")]
    WatchlistEditAddStock,
    #[strum(serialize = "watchlist.edit.add_crypto")]
    WatchlistEditAddCrypto,
    #[strum(serialize = "watchlist.edit.input_stock")]
    WatchlistEditInputStock,
    #[strum(serialize = "watchlist.edit.input_crypto")]
    WatchlistEditInputCrypto,
    #[strum(serialize = "watchlist.edit.input_rename")]
    WatchlistEditInputRename,
    #[strum(serialize = "watchlist.edit.input_ticker")]
    WatchlistEditInputTicker,
    #[strum(serialize = "agent.footer")]
    AgentFooter,
    #[strum(serialize = "agent.status.empty")]
    AgentStatusEmpty,
    #[strum(serialize = "agent.status.loading")]
    AgentStatusLoading,
    #[strum(serialize = "copy.no_data")]
    CopyNoData,
    #[strum(serialize = "copy.empty_watchlist")]
    CopyEmptyWatchlist,
    #[strum(serialize = "copy.loading")]
    CopyLoading,
    #[strum(serialize = "copy.error")]
    CopyError,
    #[strum(serialize = "copy.success")]
    CopySuccess,
    #[strum(serialize = "copy.agent_placeholder")]
    CopyAgentPlaceholder,
    #[strum(serialize = "copy.search_placeholder")]
    CopySearchPlaceholder,
    #[strum(serialize = "copy.disclaimer")]
    CopyDisclaimer,
    #[strum(serialize = "watchlist.market.pre_market")]
    WatchlistMarketPreMarket,
    #[strum(serialize = "watchlist.market.regular")]
    WatchlistMarketRegular,
    #[strum(serialize = "watchlist.market.after_hours")]
    WatchlistMarketAfterHours,
    #[strum(serialize = "calendar.empty")]
    CalendarEmpty,
    #[strum(serialize = "news.empty")]
    NewsEmpty,
    #[strum(serialize = "news.empty.watchlist_config")]
    NewsEmptyWatchlistConfig,
    #[strum(serialize = "news.empty.watchlist_matches")]
    NewsEmptyWatchlistMatches,
    #[strum(serialize = "news.footer")]
    NewsFooter,
    #[strum(serialize = "sec.footer")]
    SecFooter,
    #[strum(serialize = "news.status.loading")]
    NewsStatusLoading,
    #[strum(serialize = "news.status.error")]
    NewsStatusError,
    #[strum(serialize = "news.status.interrupted")]
    NewsStatusInterrupted,
    #[strum(serialize = "news.status.undated")]
    NewsStatusUndated,
    #[strum(serialize = "news.status.opened")]
    NewsStatusOpened,
    #[strum(serialize = "news.status.open_error")]
    NewsStatusOpenError,
    #[strum(serialize = "news.detail.source")]
    NewsDetailSource,
    #[strum(serialize = "news.detail.author")]
    NewsDetailAuthor,
    #[strum(serialize = "news.detail.published")]
    NewsDetailPublished,
    #[strum(serialize = "news.detail.priority")]
    NewsDetailPriority,
    #[strum(serialize = "news.detail.symbols")]
    NewsDetailSymbols,
    #[strum(serialize = "news.detail.link")]
    NewsDetailLink,
    #[strum(serialize = "news.detail.summary")]
    NewsDetailSummary,
    #[strum(serialize = "notes.empty")]
    NotesEmpty,
    #[strum(serialize = "notes.footer")]
    NotesFooter,
    #[strum(serialize = "notes.edit_footer")]
    NotesEditFooter,
    #[strum(serialize = "notes.delete_confirm_footer")]
    NotesDeleteConfirmFooter,
    #[strum(serialize = "notes.search_footer")]
    NotesSearchFooter,
    #[strum(serialize = "notes.detail.tags")]
    NotesDetailTags,
    #[strum(serialize = "notes.detail.tickers")]
    NotesDetailTickers,
    #[strum(serialize = "notes.detail.created")]
    NotesDetailCreated,
    #[strum(serialize = "notes.detail.updated")]
    NotesDetailUpdated,
    #[strum(serialize = "search.filter.stocks")]
    SearchFilterStocks,
    #[strum(serialize = "search.filter.etfs")]
    SearchFilterEtfs,
    #[strum(serialize = "search.help.tab_switches")]
    SearchHelpTabSwitches,
    #[strum(serialize = "search.footer")]
    SearchFooter,
    #[strum(serialize = "details.footer")]
    DetailsFooter,
    #[strum(serialize = "details.footer.compact")]
    DetailsFooterCompact,
    #[strum(serialize = "spotlight.title")]
    SpotlightTitle,
    #[strum(serialize = "spotlight.footer")]
    SpotlightFooter,
    #[strum(serialize = "search.empty")]
    SearchEmpty,
    #[strum(serialize = "search.header.symbol")]
    SearchHeaderSymbol,
    #[strum(serialize = "search.header.name")]
    SearchHeaderName,
    #[strum(serialize = "search.header.sector_industry")]
    SearchHeaderSectorIndustry,
    #[strum(serialize = "search.status.loaded")]
    SearchStatusLoaded,
    #[strum(serialize = "search.error.details_unavailable")]
    SearchErrorDetailsUnavailable,
    #[strum(serialize = "search.error.database_unavailable")]
    SearchErrorDatabaseUnavailable,
    #[strum(serialize = "details.section.quote")]
    DetailsSectionQuote,
    #[strum(serialize = "details.section.summary")]
    DetailsSectionSummary,
    #[strum(serialize = "details.section.fundamentals")]
    DetailsSectionFundamentals,
    #[strum(serialize = "details.section.details")]
    DetailsSectionDetails,
    #[strum(serialize = "details.status.loading")]
    DetailsStatusLoading,
    #[strum(serialize = "details.status.no_summary")]
    DetailsStatusNoSummary,
    #[strum(serialize = "details.label.current_price")]
    DetailsLabelCurrentPrice,
    #[strum(serialize = "details.label.change")]
    DetailsLabelChange,
    #[strum(serialize = "details.label.previous_close")]
    DetailsLabelPreviousClose,
    #[strum(serialize = "details.label.open")]
    DetailsLabelOpen,
    #[strum(serialize = "details.label.week_high")]
    DetailsLabelWeekHigh,
    #[strum(serialize = "details.label.week_low")]
    DetailsLabelWeekLow,
    #[strum(serialize = "details.label.market_cap")]
    DetailsLabelMarketCap,
    #[strum(serialize = "details.label.avg_volume")]
    DetailsLabelAvgVolume,
    #[strum(serialize = "details.label.pe_ratio")]
    DetailsLabelPeRatio,
    #[strum(serialize = "details.label.forward_pe")]
    DetailsLabelForwardPe,
    #[strum(serialize = "details.label.dividend_yield")]
    DetailsLabelDividendYield,
    #[strum(serialize = "details.label.earnings")]
    DetailsLabelEarnings,
    #[strum(serialize = "details.label.beta")]
    DetailsLabelBeta,
    #[strum(serialize = "details.label.country")]
    DetailsLabelCountry,
    #[strum(serialize = "details.label.website")]
    DetailsLabelWebsite,
    #[strum(serialize = "details.label.exchange")]
    DetailsLabelExchange,
    #[strum(serialize = "details.label.type")]
    DetailsLabelType,
    #[strum(serialize = "details.label.sector")]
    DetailsLabelSector,
    #[strum(serialize = "details.label.industry")]
    DetailsLabelIndustry,
    #[strum(serialize = "details.label.currency")]
    DetailsLabelCurrency,
    #[strum(serialize = "details.label.active")]
    DetailsLabelActive,
    #[strum(serialize = "details.label.updated")]
    DetailsLabelUpdated,
    #[strum(serialize = "details.value.days")]
    DetailsValueDays,
    #[strum(serialize = "details.chart.title")]
    DetailsChartTitle,
    #[strum(serialize = "details.chart.no_data")]
    DetailsChartNoData,
    #[strum(serialize = "details.chart.high")]
    DetailsChartHigh,
    #[strum(serialize = "details.chart.low")]
    DetailsChartLow,
    #[strum(serialize = "details.timeframe.1d")]
    DetailsTimeframeOneDay,
    #[strum(serialize = "details.timeframe.1w")]
    DetailsTimeframeOneWeek,
    #[strum(serialize = "details.timeframe.1m")]
    DetailsTimeframeOneMonth,
    #[strum(serialize = "details.timeframe.3m")]
    DetailsTimeframeThreeMonths,
    #[strum(serialize = "details.timeframe.6m")]
    DetailsTimeframeSixMonths,
    #[strum(serialize = "details.timeframe.1y")]
    DetailsTimeframeOneYear,
    #[strum(serialize = "details.timeframe.5y")]
    DetailsTimeframeFiveYears,
    #[strum(serialize = "details.timeframe.max")]
    DetailsTimeframeMax,
    #[strum(serialize = "details.label.volume")]
    DetailsLabelVolume,
    #[strum(serialize = "details.label.rvol")]
    DetailsLabelRvol,
    #[strum(serialize = "details.label.day_range")]
    DetailsLabelDayRange,
    #[strum(serialize = "details.label.headquarters")]
    DetailsLabelHeadquarters,
    #[strum(serialize = "details.label.employees")]
    DetailsLabelEmployees,
    #[strum(serialize = "details.label.after_hours")]
    DetailsLabelAfterHours,
    #[strum(serialize = "details.section.key_stats")]
    DetailsSectionKeyStats,
    #[strum(serialize = "details.section.company")]
    DetailsSectionCompany,
    #[strum(serialize = "details.section.profile")]
    DetailsSectionProfile,
    #[strum(serialize = "details.section.market_context")]
    DetailsSectionMarketContext,
    #[strum(serialize = "details.section.headlines")]
    DetailsSectionHeadlines,
    #[strum(serialize = "details.section.notes")]
    DetailsSectionNotes,
    #[strum(serialize = "details.description.expand")]
    DetailsDescriptionExpand,
    #[strum(serialize = "details.description.collapse")]
    DetailsDescriptionCollapse,
    #[strum(serialize = "details.context.expand")]
    DetailsContextExpand,
    #[strum(serialize = "details.context.loading")]
    DetailsContextLoading,
    #[strum(serialize = "details.context.backend_unavailable")]
    DetailsContextBackendUnavailable,
    #[strum(serialize = "details.context.empty")]
    DetailsContextEmpty,
    #[strum(serialize = "details.context.stale")]
    DetailsContextStale,
    #[strum(serialize = "details.context.confidence")]
    DetailsContextConfidence,
    #[strum(serialize = "details.context.cache")]
    DetailsContextCache,
    #[strum(serialize = "details.headlines.empty")]
    DetailsHeadlinesEmpty,
    #[strum(serialize = "details.headlines.no_fresh")]
    DetailsHeadlinesNoFresh,
    #[strum(serialize = "details.headlines.local_feed")]
    DetailsHeadlinesLocalFeed,
    #[strum(serialize = "details.headlines.source_unknown")]
    DetailsHeadlinesSourceUnknown,
    #[strum(serialize = "details.headlines.fresh")]
    DetailsHeadlinesFresh,
    #[strum(serialize = "details.notes.empty")]
    DetailsNotesEmpty,
    #[strum(serialize = "details.notes.unavailable")]
    DetailsNotesUnavailable,
    #[strum(serialize = "details.notes.empty_note")]
    DetailsNotesEmptyNote,
    #[strum(serialize = "metric.explanation.pe_ratio")]
    MetricExplanationPeRatio,
    #[strum(serialize = "metric.explanation.forward_pe")]
    MetricExplanationForwardPe,
    #[strum(serialize = "metric.explanation.market_cap")]
    MetricExplanationMarketCap,
    #[strum(serialize = "metric.explanation.revenue_growth")]
    MetricExplanationRevenueGrowth,
    #[strum(serialize = "metric.explanation.profit_margin")]
    MetricExplanationProfitMargin,
    #[strum(serialize = "metric.explanation.gross_margin")]
    MetricExplanationGrossMargin,
    #[strum(serialize = "metric.explanation.operating_margin")]
    MetricExplanationOperatingMargin,
    #[strum(serialize = "metric.explanation.net_margin")]
    MetricExplanationNetMargin,
    #[strum(serialize = "metric.explanation.roe")]
    MetricExplanationRoe,
    #[strum(serialize = "metric.explanation.roic")]
    MetricExplanationRoic,
    #[strum(serialize = "metric.explanation.debt_equity")]
    MetricExplanationDebtEquity,
    #[strum(serialize = "metric.explanation.ev_ebitda")]
    MetricExplanationEvEbitda,
    #[strum(serialize = "metric.explanation.beta")]
    MetricExplanationBeta,
    #[strum(serialize = "metric.explanation.volume")]
    MetricExplanationVolume,
    #[strum(serialize = "metric.explanation.dividend_yield")]
    MetricExplanationDividendYield,
    #[strum(serialize = "metric.explanation.avg_volume")]
    MetricExplanationAvgVolume,
    #[strum(serialize = "metric.explanation.relative_volume")]
    MetricExplanationRelativeVolume,
    #[strum(serialize = "metric.explanation.previous_close")]
    MetricExplanationPreviousClose,
    #[strum(serialize = "settings.title")]
    SettingsTitle,
    #[strum(serialize = "settings.footer")]
    SettingsFooter,
    #[strum(serialize = "settings.section.preferences")]
    SettingsSectionPreferences,
    #[strum(serialize = "settings.section.danger")]
    SettingsSectionDanger,
    #[strum(serialize = "settings.row.preset_ape")]
    SettingsRowPresetApe,
    #[strum(serialize = "settings.row.preset_pro")]
    SettingsRowPresetPro,
    #[strum(serialize = "settings.row.preset_custom")]
    SettingsRowPresetCustom,
    #[strum(serialize = "settings.row.experience")]
    SettingsRowExperience,
    #[strum(serialize = "settings.row.tone")]
    SettingsRowTone,
    #[strum(serialize = "settings.row.explanations")]
    SettingsRowExplanations,
    #[strum(serialize = "settings.row.agent_style")]
    SettingsRowAgentStyle,
    #[strum(serialize = "settings.row.language")]
    SettingsRowLanguage,
    #[strum(serialize = "settings.row.theme")]
    SettingsRowTheme,
    #[strum(serialize = "settings.row.onboarding")]
    SettingsRowOnboarding,
    #[strum(serialize = "settings.row.reset")]
    SettingsRowReset,
    #[strum(serialize = "settings.value.on")]
    SettingsValueOn,
    #[strum(serialize = "settings.value.off")]
    SettingsValueOff,
    #[strum(serialize = "settings.value.active")]
    SettingsValueActive,
    #[strum(serialize = "settings.value.inactive")]
    SettingsValueInactive,
    #[strum(serialize = "settings.value.custom")]
    SettingsValueCustom,
    #[strum(serialize = "settings.value.experience.simple")]
    SettingsValueExperienceSimple,
    #[strum(serialize = "settings.value.experience.pro")]
    SettingsValueExperiencePro,
    #[strum(serialize = "settings.value.tone.normal")]
    SettingsValueToneNormal,
    #[strum(serialize = "settings.value.tone.ape")]
    SettingsValueToneApe,
    #[strum(serialize = "settings.value.explanations.beginner")]
    SettingsValueExplanationsBeginner,
    #[strum(serialize = "settings.value.agent_style.chat")]
    SettingsValueAgentStyleChat,
    #[strum(serialize = "settings.value.agent_style.analyst")]
    SettingsValueAgentStyleAnalyst,
    #[strum(serialize = "dashboard.help.cycle_experience")]
    DashboardHelpCycleExperience,
    #[strum(serialize = "settings.reset.prompt")]
    SettingsResetPrompt,
    #[strum(serialize = "settings.reset.input_label")]
    SettingsResetInputLabel,
    #[strum(serialize = "settings.reset.warning")]
    SettingsResetWarning,
}

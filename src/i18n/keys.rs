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
    #[strum(serialize = "settings.title")]
    SettingsTitle,
    #[strum(serialize = "settings.footer")]
    SettingsFooter,
    #[strum(serialize = "settings.section.preferences")]
    SettingsSectionPreferences,
    #[strum(serialize = "settings.section.danger")]
    SettingsSectionDanger,
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
    #[strum(serialize = "settings.reset.prompt")]
    SettingsResetPrompt,
    #[strum(serialize = "settings.reset.input_label")]
    SettingsResetInputLabel,
    #[strum(serialize = "settings.reset.warning")]
    SettingsResetWarning,
}

import SwiftUI

struct PerformanceDetailView: View {
    let isIPad: Bool

    @EnvironmentObject private var vm: DashboardViewModel
    @State private var showRAMLine: Bool = false

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            ScrollView(showsIndicators: false) {
                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    DashboardChartsCard(
                        performanceHistory: vm.performanceHistory,
                        showRAMLine: $showRAMLine,
                        isIPad: isIPad
                    )
                }
                .padding(.horizontal, isIPad ? MSCRemoteStyle.iPadContentPadding : MSCRemoteStyle.spaceLG)
                .padding(.top, MSCRemoteStyle.spaceMD)
                .padding(.bottom, MSCRemoteStyle.spaceLG)
                .frame(maxWidth: isIPad ? MSCRemoteStyle.contentMaxWidth : .infinity)
                .frame(maxWidth: .infinity)
            }
        }
        .navigationTitle("Performance")
        .navigationBarTitleDisplayMode(.large)
        .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
        .toolbarColorScheme(.dark, for: .navigationBar)
    }
}

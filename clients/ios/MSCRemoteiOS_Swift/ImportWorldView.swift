import SwiftUI
import UniformTypeIdentifiers

/// Imports a local world ZIP as a new saved slot via P6.21's bounded
/// staged-upload routes (`phase6-api.md` §4) -- never an arbitrary remote
/// path. Mirrors `CreateWorldView`'s own shape in `WorldsView.swift`: a
/// plain (non-`async`) callback, with the caller wrapping the actual
/// network work in a `Task` -- a stored `async` closure crashes
/// AttributeGraph on presentation, per that file's own documented note.
struct ImportWorldView: View {
    @Environment(\.dismiss) private var dismiss

    let onImport: (String, Data) -> Void

    @State private var nameText: String = ""
    @State private var pickedFileName: String? = nil
    @State private var pickedData: Data? = nil
    @State private var showFilePicker: Bool = false
    @State private var errorText: String? = nil

    private var canSubmit: Bool {
        !nameText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && pickedData != nil
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                        MSCSectionHeader(title: "Import World")
                            .padding(.bottom, 4)

                        VStack(spacing: MSCRemoteStyle.spaceSM) {
                            nameField
                            filePickerRow
                        }

                        if let errorText {
                            Text(errorText)
                                .font(.system(size: 12))
                                .foregroundStyle(MSCRemoteStyle.danger)
                        }

                        Text("Pick a world ZIP exported from this or another server. It's uploaded and saved as a new slot — the live world is untouched.")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .mscCard()

                    MSCActionButton(title: "Import World", icon: "square.and.arrow.down",
                                    style: .primary, isEnabled: canSubmit) {
                        submit()
                    }

                    Spacer()
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.top, MSCRemoteStyle.spaceMD)
            }
            .navigationTitle("Import World")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Cancel") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .fileImporter(isPresented: $showFilePicker, allowedContentTypes: [.zip], allowsMultipleSelection: false) { result in
                handlePicked(result)
            }
        }
    }

    private var nameField: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Name")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
            TextField("Imported World", text: $nameText)
                .textFieldStyle(.plain)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .padding(MSCRemoteStyle.spaceSM)
                .background(MSCRemoteStyle.bgBase)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM - 2, style: .continuous))
        }
    }

    private var filePickerRow: some View {
        Button {
            showFilePicker = true
        } label: {
            HStack {
                Image(systemName: "doc.zipper")
                    .foregroundStyle(MSCRemoteStyle.accent)
                Text(pickedFileName ?? "Choose a world ZIP…")
                    .font(.system(size: 13))
                    .foregroundStyle(pickedFileName == nil ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.textPrimary)
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            .padding(MSCRemoteStyle.spaceSM)
            .background(MSCRemoteStyle.bgBase)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM - 2, style: .continuous))
        }
        .buttonStyle(.plain)
    }

    private func handlePicked(_ result: Result<[URL], Error>) {
        errorText = nil
        switch result {
        case .success(let urls):
            guard let url = urls.first else { return }
            let didStartAccess = url.startAccessingSecurityScopedResource()
            defer { if didStartAccess { url.stopAccessingSecurityScopedResource() } }
            do {
                let data = try Data(contentsOf: url)
                pickedData = data
                pickedFileName = url.lastPathComponent
                if nameText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    nameText = url.deletingPathExtension().lastPathComponent
                }
            } catch {
                errorText = "Could not read that file: \(error.localizedDescription)"
            }
        case .failure(let error):
            errorText = error.localizedDescription
        }
    }

    private func submit() {
        guard let data = pickedData else { return }
        let name = nameText.trimmingCharacters(in: .whitespacesAndNewlines)
        onImport(name, data)
        dismiss()
    }
}

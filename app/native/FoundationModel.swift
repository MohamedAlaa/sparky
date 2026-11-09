import Foundation
#if canImport(FoundationModels)
import FoundationModels

@available(macOS 26.0, *)
func classificationSchema(labels: [String]) throws -> GenerationSchema {
    let category = DynamicGenerationSchema(name: "Category",
        anyOf: ["People", "Newsletters", "Notifications", "Uncertain"])
    let label = DynamicGenerationSchema(name: "Label", anyOf: labels)
    let suggestion = DynamicGenerationSchema(name: "SuggestedCriterion", properties: [
        .init(name: "name", description: "Short new label name", schema: DynamicGenerationSchema(type: String.self)),
        .init(name: "description", description: "Matching criteria, maximum 240 characters", schema: DynamicGenerationSchema(type: String.self))
    ])
    let result = DynamicGenerationSchema(name: "EmailClassification", properties: [
        .init(name: "category", description: "Email category; Uncertain if evidence is insufficient", schema: category),
        .init(name: "labels", description: "Relevant labels; empty when uncertain",
              schema: DynamicGenerationSchema(arrayOf: label, minimumElements: 0, maximumElements: labels.count)),
        .init(name: "confidence", description: "Confidence from 0 to 1 that existing choices fit", schema: DynamicGenerationSchema(type: Double.self)),
        .init(name: "suggestions", description: "If confidence below 0.75 suggest 1-3 new label criteria; otherwise empty", schema: DynamicGenerationSchema(arrayOf: suggestion, minimumElements: 0, maximumElements: 3))
    ])
    return try GenerationSchema(root: result, dependencies: [])
}
#endif

@main struct FoundationModelBridge {
    static func main() async {
        var output: [String: Any]
        do {
            #if canImport(FoundationModels)
            if #available(macOS 26.0, *) {
                switch SystemLanguageModel.default.availability {
                case .available: break
                case .unavailable(.appleIntelligenceNotEnabled):
                    throw Failure("Enable Apple Intelligence in System Settings, then try again.")
                case .unavailable(.deviceNotEligible):
                    throw Failure("This Mac does not support Apple Intelligence. Choose Ollama or your own server in AI settings.")
                case .unavailable(.modelNotReady):
                    throw Failure("Apple’s model is still downloading or preparing. Try again when Apple Intelligence is ready.")
                default: throw Failure("Apple Foundation Model is unavailable. Choose Ollama or your own server in AI settings.")
                }
                if CommandLine.arguments.contains("--test") {
                    output = ["status": "Connected · Apple Foundation Model · on this Mac"]
                } else {
                    let data = FileHandle.standardInput.readDataToEndOfFile()
                    let input = try JSONDecoder().decode(Input.self, from: data)
                    let session = LanguageModelSession(instructions: input.system)
                    let response = try await session.respond(to: input.prompt, schema: classificationSchema(labels: input.labels),
                        options: GenerationOptions(temperature: 0, maximumResponseTokens: 512))
                    output = ["choices": [["message": ["content": response.content.jsonString]]]]
                }
            } else {
                throw Failure("Apple Foundation Model requires macOS 26 or later. Choose Ollama or your own server in AI settings.")
            }
            #else
            throw Failure("This build has no Foundation Models support. Rebuild with the macOS 26 SDK, or choose Ollama or your own server.")
            #endif
        } catch {
            output = ["error": (error as? Failure)?.message ?? "Apple Foundation Model could not classify this email. Existing labels were kept. \(error.localizedDescription)"]
        }
        if let data = try? JSONSerialization.data(withJSONObject: output) {
            FileHandle.standardOutput.write(data)
        }
    }
    struct Input: Decodable { let system: String; let prompt: String; let labels: [String] }
    struct Failure: Error { let message: String; init(_ message: String) { self.message = message } }
}

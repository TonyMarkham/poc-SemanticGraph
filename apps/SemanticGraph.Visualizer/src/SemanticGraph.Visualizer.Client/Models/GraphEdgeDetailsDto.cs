using System.Text.Json;
using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphEdgeDetailsDto(
    [property: JsonPropertyName("edgeId")] string EdgeId,
    [property: JsonPropertyName("relation")] string Relation,
    [property: JsonPropertyName("context")] string? Context,
    [property: JsonPropertyName("confidence")] string Confidence,
    [property: JsonPropertyName("confidenceScore")] double ConfidenceScore,
    [property: JsonPropertyName("weight")] double Weight,
    [property: JsonPropertyName("firstSeenRunId")] long? FirstSeenRunId,
    [property: JsonPropertyName("lastSeenRunId")] long? LastSeenRunId,
    [property: JsonPropertyName("propertiesJson")] JsonElement PropertiesJson,
    [property: JsonPropertyName("source")] GraphEdgeEndpointDto Source,
    [property: JsonPropertyName("target")] GraphEdgeEndpointDto Target,
    [property: JsonPropertyName("evidence")] IReadOnlyList<GraphEdgeEvidenceDto> Evidence);

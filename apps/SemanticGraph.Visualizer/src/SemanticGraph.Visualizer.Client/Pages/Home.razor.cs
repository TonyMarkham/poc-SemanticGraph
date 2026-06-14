using System.Text.Json;
using Blazor.Diagrams;
using Blazor.Diagrams.Core.Models.Base;
using Blazor.Diagrams.Options;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using SemanticGraph.Visualizer.Client.Components.Graph;
using SemanticGraph.Visualizer.Client.Models;
using SemanticGraph.Visualizer.Client.Services;

namespace SemanticGraph.Visualizer.Client.Pages;

public partial class Home : IDisposable
{
    private const int ProjectionLimit = 150;
    private const int SearchLimit = 25;

    private static readonly JsonSerializerOptions PrettyJsonOptions = new()
    {
        WriteIndented = true
    };

    private long _selectionRequestVersion;
    private bool _suppressSelectionEvents;

    [Inject]
    private GraphClient GraphClient { get; set; } = null!;

    private BlazorDiagram? Diagram { get; set; }

    private GraphProjectionDto? Projection { get; set; }

    private bool IsLoading { get; set; }

    private string? ErrorMessage { get; set; }

    private SemanticGraphNodeModel? SelectedNode { get; set; }

    private SemanticGraphLinkModel? SelectedEdge { get; set; }

    private GraphNodeDetailsDto? SelectedNodeDetails { get; set; }

    private GraphEdgeDetailsDto? SelectedEdgeDetails { get; set; }

    private bool IsSelectionLoading { get; set; }

    private string? SelectionErrorMessage { get; set; }

    private string SearchQuery { get; set; } = string.Empty;

    private bool IsSearching { get; set; }

    private bool HasSearchSubmitted { get; set; }

    private string? SearchErrorMessage { get; set; }

    private IReadOnlyList<GraphNodeSearchResultViewModel> SearchResults { get; set; } =
        Array.Empty<GraphNodeSearchResultViewModel>();

    private IReadOnlyDictionary<string, SemanticGraphNodeModel> VisibleNodesById { get; set; } =
        new Dictionary<string, SemanticGraphNodeModel>(StringComparer.Ordinal);

    private SemanticGraphLinkModel? CuedEdge { get; set; }

    private IReadOnlyList<SemanticGraphNodeModel> CuedEndpointNodes { get; set; } =
        Array.Empty<SemanticGraphNodeModel>();

    private bool IsEmpty => Projection is { Nodes.Count: 0 };

    private string DatabasePathText => Projection?.Metadata.DatabasePath ?? ".local/rust-workspace-extract-new.db";

    private string NodeCountText => (Projection?.Metadata.NodeCount ?? 0).ToString();

    private string EdgeCountText => (Projection?.Metadata.EdgeCount ?? 0).ToString();

    private bool ShowSearchPanel =>
        SearchErrorMessage is not null || HasSearchSubmitted || SearchResults.Count > 0;

    private string NodePropertiesJsonText => SelectedNodeDetails is null
        ? "{}"
        : FormatJson(SelectedNodeDetails.PropertiesJson);

    private string EdgePropertiesJsonText => SelectedEdgeDetails is null
        ? "{}"
        : FormatJson(SelectedEdgeDetails.PropertiesJson);

    protected override async Task OnInitializedAsync()
    {
        Diagram = CreateDiagram();
        Diagram.SelectionChanged += OnSelectionChanged;
        await LoadProjectionAsync();
    }

    public void Dispose()
    {
        if (Diagram is not null)
        {
            Diagram.SelectionChanged -= OnSelectionChanged;
        }
    }

    private async Task ReloadAsync()
    {
        await LoadProjectionAsync();
    }

    private async Task LoadProjectionAsync()
    {
        IsLoading = true;
        ErrorMessage = null;

        try
        {
            var projection = await GraphClient.GetProjectionAsync(ProjectionLimit);
            Projection = projection;

            if (Diagram is not null)
            {
                GraphDiagramBuilder.Populate(Diagram, projection);
                IndexVisibleNodes();
            }

            ClearSelectionState();
        }
        catch (Exception exception)
        {
            Projection = null;
            ErrorMessage = exception.Message;
            Diagram?.Links.Clear();
            Diagram?.Nodes.Clear();
            VisibleNodesById = new Dictionary<string, SemanticGraphNodeModel>(StringComparer.Ordinal);
            ClearSelectionState();
        }
        finally
        {
            IsLoading = false;
        }
    }

    private static BlazorDiagram CreateDiagram()
    {
        var options = new BlazorDiagramOptions
        {
            AllowMultiSelection = false,
            GridSize = 20,
            Zoom =
            {
                Minimum = 0.2
            },
            Virtualization =
            {
                Enabled = true
            }
        };

        var diagram = new BlazorDiagram(options);
        diagram.RegisterComponent<SemanticGraphNodeModel, GraphNodeWidget>();

        return diagram;
    }

    private void IndexVisibleNodes()
    {
        if (Diagram is null)
        {
            VisibleNodesById = new Dictionary<string, SemanticGraphNodeModel>(StringComparer.Ordinal);
            return;
        }

        VisibleNodesById = Diagram.Nodes
            .OfType<SemanticGraphNodeModel>()
            .ToDictionary(node => node.Id, node => node, StringComparer.Ordinal);
    }

    private void OnSelectionChanged(SelectableModel model)
    {
        if (_suppressSelectionEvents)
        {
            return;
        }

        _ = InvokeAsync(() => HandleSelectionChangedAsync(model));
    }

    private async Task HandleSelectionChangedAsync(SelectableModel model)
    {
        if (Diagram is null)
        {
            return;
        }

        ClearGraphCues();

        var selectedModel = model.Selected
            ? model
            : Diagram.GetSelectedModels().LastOrDefault();

        SelectedNode = selectedModel as SemanticGraphNodeModel;
        SelectedEdge = selectedModel as SemanticGraphLinkModel;
        SelectedNodeDetails = null;
        SelectedEdgeDetails = null;
        SelectionErrorMessage = null;

        var requestVersion = Interlocked.Increment(ref _selectionRequestVersion);

        if (SelectedEdge is not null)
        {
            ApplyEdgeCues(SelectedEdge);
        }

        if (SelectedNode is null && SelectedEdge is null)
        {
            IsSelectionLoading = false;
            StateHasChanged();
            return;
        }

        IsSelectionLoading = true;
        StateHasChanged();

        try
        {
            if (SelectedNode is not null)
            {
                var details = await GraphClient.GetNodeDetailsAsync(SelectedNode.Id);
                ApplyNodeDetails(requestVersion, details);
            }
            else if (SelectedEdge is not null)
            {
                var details = await GraphClient.GetEdgeDetailsAsync(SelectedEdge.Id);
                ApplyEdgeDetails(requestVersion, details);
            }
        }
        catch (Exception exception)
        {
            if (requestVersion == _selectionRequestVersion)
            {
                SelectionErrorMessage = exception.Message;
            }
        }
        finally
        {
            if (requestVersion == _selectionRequestVersion)
            {
                IsSelectionLoading = false;
                StateHasChanged();
            }
        }
    }

    private async Task SearchAsync()
    {
        var query = SearchQuery.Trim();

        if (query.Length == 0)
        {
            SearchResults = Array.Empty<GraphNodeSearchResultViewModel>();
            SearchErrorMessage = null;
            HasSearchSubmitted = false;
            return;
        }

        IsSearching = true;
        SearchErrorMessage = null;
        HasSearchSubmitted = true;

        try
        {
            var response = await GraphClient.SearchNodesAsync(query, SearchLimit);
            SearchResults = response.Results
                .Select(result => new GraphNodeSearchResultViewModel(
                    result,
                    VisibleNodesById.ContainsKey(result.NodeId)))
                .ToArray();
        }
        catch (Exception exception)
        {
            SearchResults = Array.Empty<GraphNodeSearchResultViewModel>();
            SearchErrorMessage = exception.Message;
        }
        finally
        {
            IsSearching = false;
        }
    }

    private async Task OnSearchKeyDown(KeyboardEventArgs args)
    {
        if (args.Key == "Enter")
        {
            await SearchAsync();
        }
    }

    private async Task SelectSearchResultAsync(GraphNodeSearchResultViewModel result)
    {
        SearchErrorMessage = null;

        if (Diagram is not null && VisibleNodesById.TryGetValue(result.Node.NodeId, out var visibleNode))
        {
            Diagram.SelectModel(visibleNode, true);
            return;
        }

        ClearDiagramSelectionSilently();
        ClearGraphCues();

        SelectedNode = null;
        SelectedEdge = null;
        SelectedNodeDetails = null;
        SelectedEdgeDetails = null;
        SelectionErrorMessage = null;

        var requestVersion = Interlocked.Increment(ref _selectionRequestVersion);
        IsSelectionLoading = true;

        try
        {
            var details = await GraphClient.GetNodeDetailsAsync(result.Node.NodeId);
            ApplyNodeDetails(requestVersion, details);
        }
        catch (Exception exception)
        {
            if (requestVersion == _selectionRequestVersion)
            {
                SelectionErrorMessage = exception.Message;
            }
        }
        finally
        {
            if (requestVersion == _selectionRequestVersion)
            {
                IsSelectionLoading = false;
            }
        }
    }

    private void ApplyNodeDetails(long requestVersion, GraphNodeDetailsDto details)
    {
        if (requestVersion != _selectionRequestVersion)
        {
            return;
        }

        SelectedNodeDetails = details;
        SelectedEdgeDetails = null;
    }

    private void ApplyEdgeDetails(long requestVersion, GraphEdgeDetailsDto details)
    {
        if (requestVersion != _selectionRequestVersion)
        {
            return;
        }

        SelectedNodeDetails = null;
        SelectedEdgeDetails = details;
    }

    private void ClearSelectionState()
    {
        Interlocked.Increment(ref _selectionRequestVersion);
        ClearGraphCues();
        SelectedNode = null;
        SelectedEdge = null;
        SelectedNodeDetails = null;
        SelectedEdgeDetails = null;
        IsSelectionLoading = false;
        SelectionErrorMessage = null;
    }

    private void ClearDiagramSelectionSilently()
    {
        if (Diagram is null)
        {
            return;
        }

        _suppressSelectionEvents = true;
        Diagram.UnselectAll();
        _suppressSelectionEvents = false;
    }

    private void ApplyEdgeCues(SemanticGraphLinkModel edge)
    {
        edge.ApplySelectedWidth(true);
        edge.SourceNode.SetRelatedEndpoint(true);
        edge.TargetNode.SetRelatedEndpoint(true);
        CuedEdge = edge;
        CuedEndpointNodes = new[] { edge.SourceNode, edge.TargetNode };
    }

    private void ClearGraphCues()
    {
        if (CuedEdge is not null)
        {
            CuedEdge.ApplySelectedWidth(false);
            CuedEdge = null;
        }

        foreach (var node in CuedEndpointNodes)
        {
            node.SetRelatedEndpoint(false);
        }

        CuedEndpointNodes = Array.Empty<SemanticGraphNodeModel>();
    }

    private static string FormatRange(
        long? startLine,
        long? startCol,
        long? endLine,
        long? endCol)
    {
        if (startLine is null || startCol is null || endLine is null || endCol is null)
        {
            return "none";
        }

        return $"{startLine}:{startCol} - {endLine}:{endCol}";
    }

    private static string FormatRange(
        long startLine,
        long startCol,
        long endLine,
        long endCol)
    {
        return $"{startLine}:{startCol} - {endLine}:{endCol}";
    }

    private static string FormatPoint(long? line, long? column)
    {
        if (line is null || column is null)
        {
            return "none";
        }

        return $"{line}:{column}";
    }

    private static string FormatOptional(string? value)
    {
        return string.IsNullOrWhiteSpace(value) ? "none" : value;
    }

    private static string FormatOptional(long? value)
    {
        return value?.ToString() ?? "none";
    }

    private static string FormatJson(JsonElement value)
    {
        return JsonSerializer.Serialize(value, PrettyJsonOptions);
    }

    private sealed record GraphNodeSearchResultViewModel(
        GraphNodeSearchResultDto Node,
        bool IsVisible);
}

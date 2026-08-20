//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'delete_application_config_workspace_request.g.dart';

/// DeleteApplicationConfigWorkspaceRequest
///
/// Properties:
/// * [bindingId]
@BuiltValue()
abstract class DeleteApplicationConfigWorkspaceRequest implements Built<DeleteApplicationConfigWorkspaceRequest, DeleteApplicationConfigWorkspaceRequestBuilder> {
  @BuiltValueField(wireName: r'binding_id')
  String get bindingId;

  DeleteApplicationConfigWorkspaceRequest._();

  factory DeleteApplicationConfigWorkspaceRequest([void updates(DeleteApplicationConfigWorkspaceRequestBuilder b)]) = _$DeleteApplicationConfigWorkspaceRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeleteApplicationConfigWorkspaceRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeleteApplicationConfigWorkspaceRequest> get serializer => _$DeleteApplicationConfigWorkspaceRequestSerializer();
}

class _$DeleteApplicationConfigWorkspaceRequestSerializer implements PrimitiveSerializer<DeleteApplicationConfigWorkspaceRequest> {
  @override
  final Iterable<Type> types = const [DeleteApplicationConfigWorkspaceRequest, _$DeleteApplicationConfigWorkspaceRequest];

  @override
  final String wireName = r'DeleteApplicationConfigWorkspaceRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeleteApplicationConfigWorkspaceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'binding_id';
    yield serializers.serialize(
      object.bindingId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeleteApplicationConfigWorkspaceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeleteApplicationConfigWorkspaceRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'binding_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.bindingId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeleteApplicationConfigWorkspaceRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeleteApplicationConfigWorkspaceRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
